// SPDX-License-Identifier: GPL-3.0-or-later
using System.Text.Json;
using System.Text.Json.Nodes;
using Monocc.Core;
int checks=0;
void Check(bool test,string name){if(!test)throw new Exception("FAIL: "+name);checks++;}
void Reject(Action action,string name){try{action();}catch(Exception e)when(e is InvalidDataException or JsonException){checks++;return;}throw new Exception("FAIL: accepted "+name);}
var pad=new PagePad();var m=new Mapping{Action="pages"};
Check(pad.Push(31,m)==false,"page down");Check(pad.Push(-90,m)==null,"holding blocks opposite push");
pad.Release(23,1);Check(pad.Push(-90,m)==null,"hysteresis rejects incomplete release");
pad.Release(20,1);Check(pad.Push(-31,m)==true,"off-center release allows immediate opposite");
pad.Release(0,1);Check(pad.Push(25,m)==null,"normal sensitivity threshold");m.Sensitivity=2;Check(pad.Push(25,m)==false,"lighter push at higher sensitivity");
pad.Release(0,2);m.Invert=true;Check(pad.Push(25,m)==true,"reverse page direction");Check(PagePad.Threshold(double.NaN)==30,"invalid sensitivity fallback");
var profile=new Profile{Name="Editor",AppPath="C:\\Apps\\Editor.exe"};profile.Mappings["uid|pressed_1"]=new(){Action="shortcut",VirtualKey=67,Modifiers=1};profile.Mappings["uid-other|pressed_1"]=new(){Action="scroll"};profile.LedColors["uid|0"]="FF0000";profile.LedColors["uid-other|0"]="00FF00";
Profiles.ResetModule(profile,"uid");Check(profile.Mappings.Count==1&&profile.Mappings.ContainsKey("uid-other|pressed_1"),"reset UID scope");Check(profile.LedColors.Count==1,"reset color scope");
Profiles.Calibrate(profile,"pad",0,150);Check(Profiles.PageAxis(profile,"pad")=="pad|joystick_y"&&profile.Mappings["pad|joystick_y"].Invert,"up calibration positive Y");
Profiles.Calibrate(profile,"pad",-150,3);Check(Profiles.PageAxis(profile,"pad")=="pad|joystick_x"&&!profile.Mappings.ContainsKey("pad|joystick_y")&&!profile.Mappings["pad|joystick_x"].Invert,"calibration replaces prior axis");
var settings=new Settings{Profiles=[profile],Orientation=270};var json=JsonSerializer.Serialize(settings,Profiles.Json);
Check(Profiles.Decode(json).Orientation==270,"settings roundtrip");Reject(()=>Profiles.Decode("{\"formatVersion\":1,\"profiles\":[]}"),"Mac backup");Reject(()=>Profiles.Decode(json.Replace("\"windows\"","\"macos\"")),"wrong platform");Reject(()=>Profiles.Decode(json.Replace("270","45")),"invalid orientation");Reject(()=>Profiles.Decode(json.Replace("\"platform\": \"windows\"","\"platform\": 123")),"invalid platform type");
var root=Path.Combine(Path.GetTempPath(),"monocc-tests-"+Guid.NewGuid().ToString("N"));
try {
 var store=new Store(root);Check(store.Load().Profiles.Count==0,"new store has no personal defaults");store.Save(settings);Check(new Store(root).Load().Profiles[0].AppPath==profile.AppPath,"atomic persistence");
 string backup=store.Backup(settings,"reset");Check(Profiles.Decode(File.ReadAllText(backup)).Profiles.Count==1,"backup content");
 File.WriteAllText(store.FilePath,"invalid original");var damaged=new Store(root);damaged.Load();Check(damaged.ReadOnly,"damaged file read-only");
 try{damaged.Save(new());throw new Exception("overwrote corrupt data");}catch(IOException){checks++;}
 Check(File.ReadAllText(store.FilePath)=="invalid original","corrupt original preserved");damaged.Import(backup,new());Check(!damaged.ReadOnly&&damaged.Load().Profiles.Count==1,"restore repairs store");
}finally{if(Directory.Exists(root))Directory.Delete(root,true);}
Check(Wire.Color("FF0000",2)==0x020000ff,"vendor low-byte red packing");Check(Wire.Color("0000FF",0)==0x00ff0000,"blue packing");Check(Wire.Axis(255,219)==-37,"signed pad axis");
var framed=Wire.Frame(new Dictionary<string,object>{{"in",new object[]{new Dictionary<string,object>{{"i",2},{"v",new object[]{125,126,0,255,219,0,0,0}}}}}});
Check(framed.Contains((byte)0x7d),"frame escapes delimiters");var parser=new Framer();var decoded=new List<JsonNode>();foreach(var b in framed)decoded.AddRange(parser.Feed([b]));Check(decoded.Count==1,"byte-by-byte fragmented framing");Check(decoded[0]["in"]![0]!["v"]![1]!.GetValue<long>()==126,"escaped payload preserved");
Check(new Framer().Feed(framed.Concat(framed).ToArray()).Count()==2,"multiple frames in chunk");Reject(()=>Wire.Decode([0x81,0xa1,0x61]),"truncated MessagePack");
var layout=JsonNode.Parse("{\"u\":\"core\",\"i\":0,\"t\":25,\"c\":[null,{\"u\":\"knobs\",\"i\":7,\"t\":9,\"c\":[]}]}" );
Check(Wire.Modules(layout).Select(x=>x.Uid).SequenceEqual(new[]{"core","knobs"}),"stable UID layout tree");
Check(new Module("buttons",3,8).Controls.Count()==3,"three buttons");Check(new Module("knobs",4,9).Controls.Count()==9,"independent knob directions and presses");
var strip=Wire.Command("write_display_slow",new object[]{true,0,0,240,4,new byte[1920]});Check(new Framer().Feed(strip).Single()["write_display_slow"]![5]!.AsArray().Count==1920,"RGB565 display strip binary");
var linuxRoot=Path.Combine(Path.GetTempPath(),"monocc-linux-tests-"+Guid.NewGuid().ToString("N"));
try {var linuxStore=new Store(linuxRoot,"linux");var linux=linuxStore.Load();Check(linux.Platform=="linux","Linux store identity");linuxStore.Save(linux);Check(Profiles.Decode(File.ReadAllText(linuxStore.FilePath),"linux").Platform=="linux","Linux settings roundtrip");Reject(()=>Profiles.Decode(File.ReadAllText(linuxStore.FilePath)),"Linux backup in Windows");Reject(()=>Profiles.Decode(json,"linux"),"Windows backup in Linux");}finally{if(Directory.Exists(linuxRoot))Directory.Delete(linuxRoot,true);}
Console.WriteLine($"PASS: {checks} shared Windows/Linux core checks (profiles, backups, reset scope, pad behavior, transport framing, layout and LED packing).");
