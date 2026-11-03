// SPDX-License-Identifier: GPL-3.0-or-later
using Control = System.Windows.Forms.Control;
using System.Text.Json.Nodes;
using System.Drawing.Imaging;
using Monocc.Core;
namespace Monocc.Windows;
internal sealed class Engine:IDisposable {
    public readonly Store Store;
    public Settings Settings {get;private set;}
    public List<Module> Modules {get;private set;}=[];
    public Profile? Active {get;private set;}
    public string Foreground {get;private set;}="";
    public string Status {get;private set;}="Waiting for your controller";
    public string Notice {get;private set;}="Connect your Monogram Core by USB. Add an app profile to begin.";
    public string? PortOverride {get;set;}
    public bool WantsConnection {get;private set;}=true;
    public string? CalibratingUid {get;set;}
    public Profile? CalibrationProfile {get;set;}
    public bool Learning {get;set;}
    public bool Connected=>link!=null&&Modules.Count>0;
    public event Action? Changed;
    public event Action<Module,string>? ControlTouched;
    readonly Control dispatch;
    readonly System.Windows.Forms.Timer timer=new(){Interval=250};
    SerialLink? link;
    DateTime retryAt,connectedAt,lastPing;
    readonly Dictionary<string,int[]> previous=[];
    readonly Dictionary<string,PagePad> pads=[];
    readonly Dictionary<string,double> accumulated=[];
    readonly Dictionary<string,DateTime> lastActions=[];
    byte[]? lastScreen;
    string? testHex;
    bool disposed;
    public Engine(Control dispatcher,string root) {
        dispatch=dispatcher;Store=new(root);Settings=Store.Load();Notice=Store.LoadError??Notice;
        timer.Tick+=(_,_)=>Tick();timer.Start();
    }
    void Post(Action action){if(!disposed&&!dispatch.IsDisposed)try{dispatch.BeginInvoke(action);}catch(InvalidOperationException){}}
    void Tick() {
        try {
            var path=Native.ForegroundPath();
            if(path!=null&&!string.Equals(path,Foreground,StringComparison.OrdinalIgnoreCase)) {
                Foreground=path;Active=Settings.Profiles.FirstOrDefault(p=>string.Equals(p.AppPath,path,StringComparison.OrdinalIgnoreCase));pads.Clear();accumulated.Clear();
                if(link!=null)Apply();Changed?.Invoke();
            }
            if(WantsConnection&&link==null&&DateTime.UtcNow>=retryAt)Connect();
            if(link!=null&&DateTime.UtcNow-lastPing>TimeSpan.FromSeconds(2)){Send("get_check",true);lastPing=DateTime.UtcNow;}
            if(link!=null&&Modules.Count==0&&DateTime.UtcNow-connectedAt>TimeSpan.FromSeconds(15))Fail("The controller did not answer. Close other controller software and check the selected USB port.");
        }catch(Exception e)when(e is IOException or UnauthorizedAccessException or InvalidOperationException){Fail(e.Message);}
    }
    void Connect() {
        retryAt=DateTime.UtcNow.AddSeconds(5);
        try {
            var ports=PortOverride is {} selected?[selected]:Native.MonogramPorts();
            if(ports.Length==0){Status="Waiting for Monogram USB connection";Changed?.Invoke();return;}
            var current=new SerialLink(ports[0]);link=current;connectedAt=DateTime.UtcNow;Modules=[];lastScreen=null;previous.Clear();pads.Clear();accumulated.Clear();
            current.Message+=node=>Post(()=>{if(link==current)Receive(node);});current.Failed+=error=>Post(()=>{if(link==current)Fail(error);});
            Status="Connecting on "+current.Port;Log(Status);
            Send("set_power_mode",0);Send("get_version",true);Send("get_layout",true);Send("get_state",true);Send("get_check",true);
            Changed?.Invoke();
        }catch(Exception e)when(e is IOException or UnauthorizedAccessException or InvalidOperationException or ArgumentException){Status="Waiting for controller";Notice="Could not connect: "+e.Message;Log(Notice);Changed?.Invoke();}
    }
    public void Reconnect(){WantsConnection=true;DisconnectLink();retryAt=DateTime.MinValue;Changed?.Invoke();}
    public void Disconnect(){WantsConnection=false;DisconnectLink();Status="Disconnected";Changed?.Invoke();}
    void DisconnectLink(){var old=link;link=null;old?.Dispose();Modules=[];previous.Clear();pads.Clear();lastScreen=null;}
    void Fail(string error){DisconnectLink();retryAt=DateTime.UtcNow.AddSeconds(5);Status="Waiting to reconnect";Notice=error;Log(error);Changed?.Invoke();}
    void Send(string name,object value)=>link?.Send(Wire.Command(name,value));
    public void Log(string text){try{Directory.CreateDirectory(Store.Root);var path=Path.Combine(Store.Root,"connection.log");if(File.Exists(path)&&new FileInfo(path).Length>2_000_000)File.Move(path,path+".previous",true);File.AppendAllText(path,$"{DateTime.Now:yyyy-MM-dd HH:mm:ss} {text}\r\n");}catch(IOException){} }
    public void Save(){try{Store.Save(Settings);Active=Settings.Profiles.FirstOrDefault(p=>string.Equals(p.AppPath,Foreground,StringComparison.OrdinalIgnoreCase));Apply();Notice="Saved automatically.";}catch(Exception e)when(e is IOException or InvalidDataException or UnauthorizedAccessException){Notice="Could not save: "+e.Message;}Changed?.Invoke();}
    public void Import(string path){Settings=Store.Import(path,Settings);Active=Settings.Profiles.FirstOrDefault(p=>string.Equals(p.AppPath,Foreground,StringComparison.OrdinalIgnoreCase));previous.Clear();pads.Clear();Apply();Changed?.Invoke();}
    public void TestLights(string? color){testHex=color;Apply();Changed?.Invoke();}
    public void FinishSetup(){testHex=null;Learning=false;CalibratingUid=null;CalibrationProfile=null;Apply();Log("Setup closed; background controller remains active.");}
    public void Reset(Profile profile,Module module){Store.Backup(Settings,"module reset");Profiles.ResetModule(profile,module.Uid);Save();}
    public void Receive(JsonNode node) {
        try {
            if(node["l"] is {} layout) {
                var next=Wire.Modules(layout).ToList();bool changed=!next.SequenceEqual(Modules);Modules=next;Status=$"{Modules.Count} units connected · {link?.Port}";
                if(changed){previous.Clear();pads.Clear();connectedAt=DateTime.UtcNow;lastScreen=null;Apply();Log(Status);Changed?.Invoke();}
            }
            if(node["in"] is JsonArray inputs)foreach(var input in inputs) {
                if(input?["i"] is not JsonValue index||input["v"] is not JsonArray values||values.Count!=8)continue;
                int i=checked((int)index.GetValue<long>());var module=Modules.FirstOrDefault(m=>m.Index==i);if(module==null)continue;
                var v=values.Select(v=>checked((int)(v?.GetValue<long>()??0))).ToArray();if(v.Any(n=>n is <0 or >255))continue;
                Input(module,v);
            }
        }catch(Exception e)when(e is InvalidOperationException or OverflowException or ArgumentException){Log("Ignored malformed controller message: "+e.Message);}
    }
    PagePad Pad(string uid){if(!pads.TryGetValue(uid,out var pad))pads[uid]=pad=new();return pad;}
    void Input(Module module,int[] v) {
        previous.TryGetValue(module.Uid,out var prior);previous[module.Uid]=v;
        if(module.Kind==24) {
            int x=Wire.Axis(v[1],v[2]),y=Wire.Axis(v[3],v[4]);
            if(CalibratingUid==module.Uid&&CalibrationProfile is {} p&&Math.Max(Math.Abs(x),Math.Abs(y))>80){Profiles.Calibrate(p,module.Uid,x,y);CalibratingUid=null;CalibrationProfile=null;Save();Notice="Pad direction learned. Release the pad, close setup and try it in your app.";Changed?.Invoke();return;}
            if(Active is {} active&&Profiles.PageAxis(active,module.Uid) is {} axis&&active.Mappings.TryGetValue(axis,out var mapping))Pad(module.Uid).Release(axis.EndsWith("joystick_x",StringComparison.Ordinal)?x:y,mapping.Sensitivity);
            int delta=unchecked((sbyte)v[0]);if(delta!=0)Dispatch(module,delta>0?"rotation_right":"rotation_left",delta,prior);
            if(x!=0)Dispatch(module,"joystick_x",x,prior);if(y!=0)Dispatch(module,"joystick_y",y,prior);return;
        }
        if(module.Kind is 25 or 8 or 9)for(int n=1;n<=(module.Kind==25?2:3);n++)if((v[0]&(1<<(n-1)))!=0&&((prior?[0]??0)&(1<<(n-1)))==0)Dispatch(module,$"pressed_{n}",1,prior);
        if(module.Kind==9)for(int n=1;n<=3;n++){int d=unchecked((sbyte)v[n]);if(d!=0)Dispatch(module,$"rotation_{(d>0?"right":"left")}_{n}",d,prior);}
        if(module.Kind==10)for(int n=1;n<=3;n++)if(prior!=null&&v[n]!=prior[n])Dispatch(module,$"set_{n}",v[n],prior);
    }
    void Dispatch(Module module,string evt,double value,int[]? prior) {
        if(Learning){Learning=false;ControlTouched?.Invoke(module,evt);Changed?.Invoke();return;}
        if(CalibratingUid!=null||DateTime.UtcNow-connectedAt<TimeSpan.FromSeconds(1)||!Settings.ActionsEnabled||Active==null)return;
        var path=Native.ForegroundPath();if(path==null||!string.Equals(path,Active.AppPath,StringComparison.OrdinalIgnoreCase))return;
        string key=Profiles.Key(module.Uid,evt);if(!Active.Mappings.TryGetValue(key,out var m))return;
        if(m.Action=="pages") {
            if(Profiles.PageAxis(Active,module.Uid)!=key)return;
            if(Pad(module.Uid).Push(value,m) is bool up)CheckSend(Native.Shortcut(up?0x21:0x22,0));return;
        }
        double delta=value;
        if(evt.StartsWith("set_",StringComparison.Ordinal)){if(prior==null)return;int n=int.Parse(evt[4..]);delta=value-prior[n];}
        delta*=m.Invert?-1:1;
        if(m.Action=="scroll")CheckSend(Native.Scroll((int)Math.Clamp(delta*m.Sensitivity*120,-2400,2400)));
        if(m.Action=="volume"&&module.Kind==10){try{Audio.SetVolume((float)(m.Invert?255-value:value)/255);}catch(Exception e){Notice="Windows audio device unavailable: "+e.Message;Changed?.Invoke();}return;}
        if(m.Action=="shortcut") {
            int count=1;
            if(!evt.StartsWith("pressed_",StringComparison.Ordinal)) {accumulated.TryGetValue(key,out var a);a+=Math.Abs(delta)*m.Sensitivity*(module.Kind==10?.1:1);count=(int)Math.Min(8,a);accumulated[key]=a-count;if(count==0)return;}
            if(lastActions.TryGetValue(key,out var time)&&DateTime.UtcNow-time<TimeSpan.FromMilliseconds(50))return;
            lastActions[key]=DateTime.UtcNow;for(int n=0;n<count;n++)CheckSend(Native.Shortcut(m.VirtualKey,m.Modifiers));
        }
    }
    void CheckSend(bool sent){if(!sent){Notice="Windows blocked this action. A standard app cannot control an app running as administrator. Check Troubleshooting in setup.";Changed?.Invoke();Log(Notice);}}
    public void Apply() {
        if(link==null)return;
        foreach(var module in Modules) {
            int channels=module.Kind is 8 or 9 or 10?3:1;
            for(int sub=0;sub<channels;sub++){string hex=testHex??Active?.LedColors.GetValueOrDefault(Profiles.Key(module.Uid,sub.ToString()))??"FFFFFF";Send("set_module",new object[]{module.Index,channels==3?5:2,Wire.Color(hex,sub)});}
            if(channels==3)Send("set_module",new object[]{module.Index,5,127L<<24});
        }
        using var screen=ScreenImage(Active,Foreground,Settings.Orientation);
        var bytes=Pixels(screen);if(lastScreen!=null&&bytes.SequenceEqual(lastScreen))return;
        for(int y=0;y<240;y+=4)link.Send(Wire.Command("write_display_slow",new object[]{true,0,y,240,y+4,bytes[(y*480)..((y+4)*480)]}));
        Send("invoke_display",3);Send("invoke_display",4);lastScreen=bytes;
    }
    public static Bitmap ScreenImage(Profile? profile,string foreground,int orientation) {
        var bitmap=new Bitmap(240,240,PixelFormat.Format24bppRgb);
        using(var g=Graphics.FromImage(bitmap)) {
            g.Clear(Color.FromArgb(9,13,20));g.SmoothingMode=System.Drawing.Drawing2D.SmoothingMode.HighQuality;
            Image? image=null;
            try{if(profile?.IconPng is {} png){using var stream=new MemoryStream(Convert.FromBase64String(png));using var decoded=Image.FromStream(stream);image=new Bitmap(decoded);}else if(File.Exists(foreground)){using var icon=Icon.ExtractAssociatedIcon(foreground);image=icon?.ToBitmap();}}catch(Exception e)when(e is ArgumentException or FormatException or IOException){}
            using(image)if(profile?.Display!="name"&&image!=null){int size=profile?.Display=="icon"?180:124;g.DrawImage(image,(240-size)/2,profile?.Display=="icon"?30:25,size,size);}
            if(profile?.Display!="icon"||image==null){using var font=new Font("Segoe UI",18,FontStyle.Bold);using var brush=new SolidBrush(Color.White);var rect=new RectangleF(10,profile?.Display=="name"?65:160,220,72);using var format=new StringFormat{Alignment=StringAlignment.Center,LineAlignment=StringAlignment.Center,Trimming=StringTrimming.EllipsisWord};g.DrawString(profile?.Name??(string.IsNullOrEmpty(foreground)?"MONOcc":Path.GetFileNameWithoutExtension(foreground)),font,brush,rect,format);}
        }
        bitmap.RotateFlip(orientation switch{90=>RotateFlipType.Rotate90FlipNone,180=>RotateFlipType.Rotate180FlipNone,270=>RotateFlipType.Rotate270FlipNone,_=>RotateFlipType.RotateNoneFlipNone});return bitmap;
    }
    static byte[] Pixels(Bitmap image) {
        var data=new byte[115200];for(int y=0;y<240;y++)for(int x=0;x<240;x++){var c=image.GetPixel(x,y);ushort p=(ushort)(((c.R>>3)<<11)|((c.G>>2)<<5)|(c.B>>3));int offset=(y*240+x)*2;data[offset]=(byte)(p>>8);data[offset+1]=(byte)p;}return data;
    }
    public void Dispose(){disposed=true;timer.Stop();timer.Dispose();DisconnectLink();}
}
