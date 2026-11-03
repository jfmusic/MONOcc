// SPDX-License-Identifier: GPL-3.0-or-later
using Avalonia.Threading;
using System.Text.Json.Nodes;
using Monocc.Core;
namespace Monocc.Linux;
internal sealed class Engine:IDisposable {
    public Store Store {get;}
    public Settings Settings {get;private set;}
    public List<Module> Modules {get;private set;}=[];
    public Profile? Active {get;private set;}
    public string Foreground {get;private set;}="";
    public string Status {get;private set;}="Waiting for Monogram Core USB connection";
    public string Notice {get;private set;}="Linux contributor preview: screen upload and Wayland support remain to be implemented.";
    public string Session=>desktop.Session;
    public string? CalibratingUid {get;set;}
    public Profile? CalibrationProfile {get;set;}
    public event Action? Changed;
    readonly Desktop desktop=new();readonly DispatcherTimer timer=new(){Interval=TimeSpan.FromMilliseconds(250)};
    readonly Dictionary<string,int[]> previous=[];readonly Dictionary<string,PagePad> pads=[];
    SerialLink? link;DateTime retry,connected,ping;bool disposed;
    public Engine() {
        string root=Environment.GetEnvironmentVariable("XDG_CONFIG_HOME") is {} xdg&&Path.IsPathRooted(xdg)?Path.Combine(xdg,"MONOcc"):Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.UserProfile),".config","MONOcc");
        Store=new(root,"linux");Settings=Store.Load();if(Store.LoadError!=null)Notice=Store.LoadError;
        timer.Tick+=(_,_)=>Tick();timer.Start();
    }
    void Tick() {
        var path=desktop.ForegroundPath();if(path!=null&&!string.Equals(path,Foreground,StringComparison.Ordinal)){Foreground=path;RefreshActive();pads.Clear();Apply();Changed?.Invoke();}
        if(link==null&&DateTime.UtcNow>=retry)Connect();
        if(link!=null&&DateTime.UtcNow-ping>TimeSpan.FromSeconds(2)){Send("get_check",true);ping=DateTime.UtcNow;}
        if(link!=null&&Modules.Count==0&&DateTime.UtcNow-connected>TimeSpan.FromSeconds(15))Fail("The Core did not answer. Close other controller apps and reconnect USB.");
    }
    void RefreshActive()=>Active=Settings.Profiles.FirstOrDefault(p=>string.Equals(p.AppPath,Foreground,StringComparison.Ordinal));
    void Connect() {
        retry=DateTime.UtcNow.AddSeconds(5);
        try {
            string? port=Desktop.Ports().FirstOrDefault();if(port==null)return;
            var current=new SerialLink(port);link=current;connected=DateTime.UtcNow;previous.Clear();pads.Clear();
            current.Message+=n=>Dispatcher.UIThread.Post(()=>{if(!disposed&&link==current)Receive(n);});current.Failed+=error=>Dispatcher.UIThread.Post(()=>{if(!disposed&&link==current)Fail(error);});
            Status="Connecting on "+port;Send("set_power_mode",0);Send("get_layout",true);Send("get_state",true);Send("get_check",true);Changed?.Invoke();
        }catch(Exception e)when(e is IOException or UnauthorizedAccessException or ArgumentException or InvalidOperationException){Notice="USB connection unavailable: "+e.Message+". The Debian package includes a targeted USB access rule; reconnect the Core after installation.";Changed?.Invoke();}
    }
    void Send(string key,object value)=>link?.Send(Wire.Command(key,value));
    void Fail(string error){var old=link;link=null;old?.Dispose();Modules=[];previous.Clear();pads.Clear();retry=DateTime.UtcNow.AddSeconds(5);Status="Waiting to reconnect";Notice=error;Log(error);Changed?.Invoke();}
    public void Reconnect(){Fail("Reconnecting controller…");retry=DateTime.MinValue;}
    public void Save(){try{Store.Save(Settings);RefreshActive();Apply();Notice="Saved automatically.";}catch(Exception e){Notice="Could not save: "+e.Message;}Changed?.Invoke();}
    public void Import(string path){Settings=Store.Import(path,Settings);RefreshActive();pads.Clear();Apply();Changed?.Invoke();}
    public void Reset(Profile p,Module m){Store.Backup(Settings,"module reset");Profiles.ResetModule(p,m.Uid);Save();}
    public void FinishSetup(){CalibratingUid=null;CalibrationProfile=null;Log("Setup closed; Linux controller remains running.");}
    void Log(string text){try{Directory.CreateDirectory(Store.Root);var path=Path.Combine(Store.Root,"connection.log");if(File.Exists(path)&&new FileInfo(path).Length>1_000_000)File.Move(path,path+".previous",true);File.AppendAllText(path,$"{DateTime.Now:O} {text}\n");}catch(IOException){} }
    void Receive(JsonNode node) {
        try {
            if(node["l"] is {} layout){var next=Wire.Modules(layout).ToList();if(!next.SequenceEqual(Modules)){Modules=next;previous.Clear();pads.Clear();connected=DateTime.UtcNow;Status=$"{Modules.Count} units connected";Apply();Changed?.Invoke();}}
            if(node["in"] is JsonArray inputs)foreach(var input in inputs) {
                if(input?["i"] is not JsonValue idx||input["v"] is not JsonArray values||values.Count!=8)continue;
                var module=Modules.FirstOrDefault(m=>m.Index==idx.GetValue<long>());if(module==null)continue;
                var v=values.Select(n=>(int)(n?.GetValue<long>()??0)).ToArray();if(v.Any(n=>n is <0 or >255))continue;Input(module,v);
            }
        }catch(Exception e)when(e is InvalidOperationException or ArgumentException or OverflowException){Log("Ignored malformed message: "+e.Message);}
    }
    PagePad Pad(string uid){if(!pads.TryGetValue(uid,out var p))pads[uid]=p=new();return p;}
    void Input(Module m,int[] v) {
        previous.TryGetValue(m.Uid,out var prior);previous[m.Uid]=v;
        if(m.Kind==24) {
            int x=Wire.Axis(v[1],v[2]),y=Wire.Axis(v[3],v[4]);
            if(CalibratingUid==m.Uid&&CalibrationProfile is {} p&&Math.Max(Math.Abs(x),Math.Abs(y))>80){Profiles.Calibrate(p,m.Uid,x,y);CalibratingUid=null;CalibrationProfile=null;Save();return;}
            if(Active is {} a&&Profiles.PageAxis(a,m.Uid) is {} axis&&a.Mappings.TryGetValue(axis,out var mapping))Pad(m.Uid).Release(axis.EndsWith("joystick_x",StringComparison.Ordinal)?x:y,mapping.Sensitivity);
            int d=unchecked((sbyte)v[0]);if(d!=0)Dispatch(m,d>0?"rotation_right":"rotation_left",d);
            if(x!=0)Dispatch(m,"joystick_x",x);if(y!=0)Dispatch(m,"joystick_y",y);return;
        }
        if(m.Kind is 25 or 8 or 9)for(int n=1;n<=(m.Kind==25?2:3);n++)if((v[0]&(1<<(n-1)))!=0&&((prior?[0]??0)&(1<<(n-1)))==0)Dispatch(m,$"pressed_{n}",1);
        if(m.Kind==9)for(int n=1;n<=3;n++){int d=unchecked((sbyte)v[n]);if(d!=0)Dispatch(m,$"rotation_{(d>0?"right":"left")}_{n}",d);}
        if(m.Kind==10&&prior!=null)for(int n=1;n<=3;n++)if(v[n]!=prior[n])Dispatch(m,$"set_{n}",v[n]-prior[n]);
    }
    void Dispatch(Module module,string evt,double value) {
        if(!desktop.Supported||!Settings.ActionsEnabled||CalibratingUid!=null||Active==null||DateTime.UtcNow-connected<TimeSpan.FromSeconds(1)||desktop.ForegroundPath()!=Active.AppPath)return;
        string key=Profiles.Key(module.Uid,evt);if(!Active.Mappings.TryGetValue(key,out var mapping))return;
        if(mapping.Action=="pages"){if(Profiles.PageAxis(Active,module.Uid)==key&&Pad(module.Uid).Push(value,mapping) is bool up)desktop.Shortcut(up?0x21:0x22,0);return;}
        if(mapping.Action=="scroll")desktop.Scroll(value*mapping.Sensitivity*(mapping.Invert?-1:1));
        if(mapping.Action=="shortcut")desktop.Shortcut(mapping.VirtualKey,mapping.Modifiers);
    }
    void Apply() {
        if(link==null)return;
        foreach(var m in Modules){int channels=m.Kind is 8 or 9 or 10?3:1;for(int sub=0;sub<channels;sub++){string hex=Active?.LedColors.GetValueOrDefault(Profiles.Key(m.Uid,sub.ToString()))??"FFFFFF";Send("set_module",new object[]{m.Index,channels==3?5:2,Wire.Color(hex,sub)});}if(channels==3)Send("set_module",new object[]{m.Index,5,127L<<24});}
    }
    public void Dispose(){disposed=true;timer.Stop();var old=link;link=null;old?.Dispose();desktop.Dispose();}
}
