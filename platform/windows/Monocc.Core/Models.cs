// SPDX-License-Identifier: GPL-3.0-or-later
using System.Text.Json;
using System.Text.Json.Serialization;

namespace Monocc.Core;
public sealed class Mapping {
    public string Label {get;set;}="";
    public string Action {get;set;}="none";
    public int VirtualKey {get;set;}
    public int Modifiers {get;set;} // 1 Ctrl, 2 Alt, 4 Shift, 8 Windows
    public string Shortcut {get;set;}="";
    public double Sensitivity {get;set;}=1;
    public bool Invert {get;set;}
}
public sealed class Profile {
    public string Id {get;set;}=Guid.NewGuid().ToString();
    public string Name {get;set;}="";
    public string AppPath {get;set;}="";
    public string Display {get;set;}="both";
    public string? IconPng {get;set;}
    public Dictionary<string,Mapping> Mappings {get;set;}=[];
    public Dictionary<string,string> LedColors {get;set;}=[];
}
public sealed class Settings {
    public int FormatVersion {get;set;}=1;
    public string Platform {get;set;}="windows";
    public List<Profile> Profiles {get;set;}=[];
    public Dictionary<string,string> UnitNames {get;set;}=[];
    public Dictionary<string,bool> SliderFlipped {get;set;}=[];
    public Dictionary<string,double[]> WhiteBalance {get;set;}=[];
    public bool ActionsEnabled {get;set;}=true;
    public int Orientation {get;set;}
    public string SelectedProfile {get;set;}="";
}
public sealed record Control(string Event,string Title);
public sealed record Module(string Uid,int Index,int Kind) {
    public string Name=>Kind switch {25=>"Core",8=>"Tri-Button",9=>"Tri-Dial",10=>"Tri-Slider",24=>"Orbiter",_=>"Unsupported module"};
    public IEnumerable<Control> Controls=>Kind switch {
        25=>Enumerable.Range(1,2).Select(n=>new Control($"pressed_{n}",$"Button {n}")),
        8=>Enumerable.Range(1,3).Select(n=>new Control($"pressed_{n}",$"Button {n}")),
        10=>Enumerable.Range(1,3).Select(n=>new Control($"set_{n}",$"Slider {n}")),
        9=>Enumerable.Range(1,3).SelectMany(n=>new[]{new Control($"rotation_right_{n}",$"Knob {n} · clockwise"),new Control($"rotation_left_{n}",$"Knob {n} · counterclockwise"),new Control($"pressed_{n}",$"Knob {n} · press")}),
        24=>[new("rotation_right","Ring · clockwise"),new("rotation_left","Ring · counterclockwise"),new("joystick_x","Pad · axis X"),new("joystick_y","Pad · axis Y")],_=>[]};
}
public static class Profiles {
    public static readonly JsonSerializerOptions Json=new(){PropertyNamingPolicy=JsonNamingPolicy.CamelCase,WriteIndented=true};
    public static string Key(string uid,string evt)=>uid+"|"+evt;
    public static string? PageAxis(Profile p,string uid)=>p.Mappings.Where(x=>x.Key.StartsWith(uid+"|joystick_",StringComparison.Ordinal)&&x.Value.Action=="pages").OrderByDescending(x=>x.Value.Label=="Pad up/down · one page").Select(x=>x.Key).FirstOrDefault();
    public static void Validate(Settings s) {
        if(s.FormatVersion!=1||!new[]{"windows","linux"}.Contains(s.Platform))throw new InvalidDataException("Choose a backup for this platform. Shortcuts from another operating system must be recreated.");
        if(s.Profiles is null||s.UnitNames is null||s.WhiteBalance is null||s.SliderFlipped is null||s.Profiles.Count>500)throw new InvalidDataException("Invalid backup contents.");
        if(!new[]{0,90,180,270}.Contains(s.Orientation))throw new InvalidDataException("Invalid console rotation.");
        var ids=new HashSet<string>();
        foreach(var p in s.Profiles) {
            if(p is null||string.IsNullOrWhiteSpace(p.Id)||!ids.Add(p.Id)||string.IsNullOrWhiteSpace(p.Name)||string.IsNullOrWhiteSpace(p.AppPath)||p.Mappings is null||p.LedColors is null||p.IconPng?.Length>1_000_000)throw new InvalidDataException("Invalid app profile.");
            foreach(var m in p.Mappings.Values)if(m is null||!new[]{"none","shortcut","scroll","pages","volume"}.Contains(m.Action)||m.VirtualKey is <0 or >255||m.Modifiers is <0 or >15||!double.IsFinite(m.Sensitivity)||m.Sensitivity is <0.25 or >4)throw new InvalidDataException("Invalid control assignment.");
            foreach(var color in p.LedColors.Values)if(color is null||color.Length!=6||!uint.TryParse(color,System.Globalization.NumberStyles.HexNumber,null,out _))throw new InvalidDataException("Invalid light color.");
        }
        foreach(var gains in s.WhiteBalance.Values)if(gains is null||gains.Length!=3||gains.Any(n=>!double.IsFinite(n)||n<0||n>1))throw new InvalidDataException("Invalid light correction.");
    }
    public static Settings Decode(string text,string expectedPlatform="windows") {
        using var doc=JsonDocument.Parse(text);
        if(doc.RootElement.ValueKind!=JsonValueKind.Object||!doc.RootElement.TryGetProperty("platform",out var platform)||platform.ValueKind!=JsonValueKind.String||platform.GetString()!=expectedPlatform)throw new InvalidDataException("Choose a backup for this platform. Shortcuts from another operating system must be recreated.");
        var s=JsonSerializer.Deserialize<Settings>(text,Json)??throw new InvalidDataException("Empty backup.");Validate(s);return s;
    }
    public static void ResetModule(Profile p,string uid) {
        foreach(var key in p.Mappings.Keys.Where(k=>k.StartsWith(uid+"|",StringComparison.Ordinal)).ToArray())p.Mappings.Remove(key);
        foreach(var key in p.LedColors.Keys.Where(k=>k.StartsWith(uid+"|",StringComparison.Ordinal)).ToArray())p.LedColors.Remove(key);
    }
    public static void Calibrate(Profile p,string uid,int x,int y) {
        var evt=Math.Abs(x)>Math.Abs(y)?"joystick_x":"joystick_y";var value=evt=="joystick_x"?x:y;
        foreach(var key in p.Mappings.Where(k=>k.Key.StartsWith(uid+"|joystick_",StringComparison.Ordinal)&&k.Value.Action=="pages").Select(k=>k.Key).ToArray())p.Mappings.Remove(key);
        p.Mappings[Key(uid,evt)]=new(){Action="pages",Label="Pad up/down · one page",Invert=value>0};
    }
}
public sealed class Store(string root,string platform="windows") {
    public string Root {get;}=root;
    public string FilePath=>Path.Combine(Root,"profiles.json");
    public bool ReadOnly {get;private set;}
    public string? LoadError {get;private set;}
    public Settings Load() {
        Directory.CreateDirectory(Root);
        if(!File.Exists(FilePath))return new(){Platform=platform};
        try{return Profiles.Decode(File.ReadAllText(FilePath),platform);}
        catch(Exception e) when(e is IOException or JsonException or InvalidDataException) {ReadOnly=true;LoadError="Saved settings could not be read. The original file is preserved. Restore a backup before editing. "+e.Message;return new(){Platform=platform};}
    }
    public void Save(Settings s) {
        if(ReadOnly)throw new IOException(LoadError);
        if(s.Platform!=platform)throw new InvalidDataException("Backup platform does not match this app.");
        Profiles.Validate(s);Atomic(FilePath,JsonSerializer.Serialize(s,Profiles.Json));
    }
    public string Backup(Settings s,string reason) {
        Directory.CreateDirectory(Root);var path=Path.Combine(Root,$"Before {reason} {DateTime.Now:yyyyMMdd-HHmmss}-{Guid.NewGuid():N}.json");
        Atomic(path,ReadOnly&&File.Exists(FilePath)?File.ReadAllText(FilePath):JsonSerializer.Serialize(s,Profiles.Json));return path;
    }
    public Settings Import(string path,Settings old) {
        if(new FileInfo(path).Length>20_000_000)throw new InvalidDataException("Backup is too large.");
        var candidate=Profiles.Decode(File.ReadAllText(path),platform);Backup(old,"restore");
        Atomic(FilePath,JsonSerializer.Serialize(candidate,Profiles.Json));ReadOnly=false;LoadError=null;return candidate;
    }
    static void Atomic(string path,string value) {
        Directory.CreateDirectory(Path.GetDirectoryName(path)!);var tmp=path+"."+Guid.NewGuid().ToString("N")+".tmp";
        try{File.WriteAllText(tmp,value);File.Move(tmp,path,true);}finally{if(File.Exists(tmp))File.Delete(tmp);}
    }
}
public sealed class PagePad {
    bool held;
    public static double Threshold(double gain)=>15+15/Math.Clamp(double.IsFinite(gain)?gain:1,.25,4);
    public void Release(double axis,double gain){if(Math.Abs(axis)<=Math.Min(24,Threshold(gain)*.75))held=false;}
    public bool? Push(double value,Mapping m) {
        if(held||Math.Abs(value)<=Threshold(m.Sensitivity))return null;
        held=true;return(m.Invert?-value:value)<0;
    }
    public void Clear()=>held=false;
}
