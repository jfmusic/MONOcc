// SPDX-License-Identifier: GPL-3.0-or-later
using Avalonia;
using Avalonia.Controls;
using Avalonia.Controls.ApplicationLifetimes;
using Avalonia.Media;
using Avalonia.Platform;
using Avalonia.Themes.Fluent;
using Avalonia.Threading;
using System.Diagnostics;
using System.IO.Pipes;
using Monocc.Core;
namespace Monocc.Linux;
internal static class Program {
    static FileStream? lease;
    public static string PipeName=>"MONOcc.Linux."+Environment.UserName;
    [STAThread]static void Main(string[] args) {
        string runtime=Environment.GetEnvironmentVariable("XDG_RUNTIME_DIR")??Path.Combine(Path.GetTempPath(),"MONOcc-"+Environment.UserName);
        Directory.CreateDirectory(runtime);string path=Path.Combine(runtime,"MONOcc.lock");
        try{lease=new(path,FileMode.OpenOrCreate,FileAccess.ReadWrite,FileShare.None);}catch(IOException){if(!args.Contains("--background"))try{using var pipe=new NamedPipeClientStream(".",PipeName,PipeDirection.Out);pipe.Connect(1500);pipe.WriteByte(1);}catch(IOException){}catch(TimeoutException){}return;}
        using(lease)AppBuilder.Configure<App>().UsePlatformDetect().LogToTrace().StartWithClassicDesktopLifetime(args,ShutdownMode.OnExplicitShutdown);
    }
}
public sealed class App:Application {
    Engine? engine;IClassicDesktopStyleApplicationLifetime? lifetime;TrayIcon? tray;Setup? setup;readonly CancellationTokenSource cancel=new();
    public override void Initialize()=>Styles.Add(new FluentTheme());
    public override void OnFrameworkInitializationCompleted() {
        if(ApplicationLifetime is IClassicDesktopStyleApplicationLifetime desktop){lifetime=desktop;engine=new();
            tray=new(){ToolTipText="MONOcc Linux starting point",Icon=new WindowIcon(AssetLoader.Open(new Uri("avares://MONOcc/MONOcc.ico")))};
            var menu=new NativeMenu();var open=new NativeMenuItem("Open Setup…");open.Click+=(_,_)=>ShowSetup();menu.Items.Add(open);var stop=new NativeMenuItem("Stop MONOcc and Exit");stop.Click+=(_,_)=>desktop.Shutdown();menu.Items.Add(stop);tray.Menu=menu;tray.Clicked+=(_,_)=>ShowSetup();TrayIcon.SetIcons(this,new TrayIcons{tray});
            desktop.Exit+=(_,_)=>{cancel.Cancel();engine.Dispose();tray.Dispose();};_=Listen();
            if(!desktop.Args!.Contains("--background"))ShowSetup();
            if(desktop.Args.Contains("--smoke-test"))DispatcherTimer.RunOnce(()=>desktop.Shutdown(),TimeSpan.FromSeconds(2));
        }base.OnFrameworkInitializationCompleted();
    }
    void ShowSetup(){if(engine==null)return;if(setup==null){setup=new(engine);setup.Closed+=(_,_)=>{engine.FinishSetup();setup=null;};}setup.Show();setup.Activate();}
    async Task Listen(){while(!cancel.IsCancellationRequested)try{using var pipe=new NamedPipeServerStream(Program.PipeName,PipeDirection.In,1,PipeTransmissionMode.Byte,PipeOptions.Asynchronous|PipeOptions.CurrentUserOnly);await pipe.WaitForConnectionAsync(cancel.Token);var buffer=new byte[1];if(await pipe.ReadAsync(buffer,cancel.Token)>0)Dispatcher.UIThread.Post(ShowSetup);}catch(OperationCanceledException){break;}catch(IOException){await Task.Delay(500);}}
}
internal sealed class Setup:Window {
    readonly Engine engine;readonly ComboBox apps=new(){Width=240},modules=new(){Width=240},controls=new(){Width=280};
    readonly ComboBox actions=new(){Width=280};readonly ComboBox shortcuts=new(){Width=280};readonly CheckBox reverse=new(){Content="Reverse direction"};readonly NumericUpDown sensitivity=new(){Minimum=.25m,Maximum=4,Increment=.25m,Value=1,Width=120};readonly TextBlock status=new(){TextWrapping=TextWrapping.Wrap};
    bool updating;
    Profile? Profile=>apps.SelectedItem is Choice<Profile> p?p.Value:null;
    Module? Module=>modules.SelectedItem is Choice<Module> m?m.Value:null;
    Monocc.Core.Control? Selected=>controls.SelectedItem is Choice<Monocc.Core.Control> c?c.Value:null;
    string? Key=>Module is {} m&&Selected is {} c?Profiles.Key(m.Uid,c.Event):null;
    Mapping Mapping=>Key is {} key&&Profile?.Mappings.TryGetValue(key,out var m)==true?m:new();
    static Button Button(string title,Action action){var b=new Button{Content=title,Margin=new(0,5),MinWidth=180};b.Click+=(_,_)=>action();return b;}
    static TextBlock Text(string text)=>new(){Text=text,TextWrapping=TextWrapping.Wrap,MaxWidth=730,Margin=new(0,5)};
    static readonly (string,int,int)[] Presets=[("Ctrl+A · Select all",65,1),("Ctrl+C · Copy",67,1),("Ctrl+V · Paste",86,1),("Ctrl+S · Save",83,1),("Ctrl+Z · Undo",90,1),("Space",32,0),("Return",13,0),("Page Up",0x21,0),("Page Down",0x22,0)];
    string signature="";
    public Setup(Engine e) {
        engine=e;Title="MONOcc v0.2.0 · Linux contributor preview";Width=900;Height=800;MinWidth=700;MinHeight=600;
        var panel=new StackPanel{Margin=new(22),Spacing=7};Content=new ScrollViewer{Content=panel};panel.Children.Add(new TextBlock{Text="MONOcc · Linux starting point",FontSize=25,FontWeight=FontWeight.Bold});panel.Children.Add(Text(engine.Session));panel.Children.Add(status);
        panel.Children.Add(Text("CoreV2 USB, app profiles, common shortcuts, ring scroll, pad pages, LEDs and background operation are implemented here as an initial port. Screen icons, screen rotation, general shortcut recording, audio volume and Wayland support are not implemented yet."));
        panel.Children.Add(Text("1. Open the app you want to control. Return here and add it. 2. Select a unit and control. 3. Assign an action. Close setup to keep the controller running."));
        panel.Children.Add(Button("Use last active app",()=>{if(string.IsNullOrEmpty(engine.Foreground)){SetStatus("Open a target app in an X11 session, then return to setup.");return;}var p=engine.Settings.Profiles.FirstOrDefault(p=>p.AppPath==engine.Foreground);if(p==null){p=new(){Name=Path.GetFileName(engine.Foreground),AppPath=engine.Foreground};engine.Settings.Profiles.Add(p);}engine.Settings.SelectedProfile=p.Id;engine.Save();RefreshProfiles();}));
        panel.Children.Add(apps);panel.Children.Add(modules);panel.Children.Add(controls);
        actions.ItemsSource=new[]{"Unassigned","Keyboard shortcut","Scroll","Page up / down"};panel.Children.Add(actions);
        shortcuts.ItemsSource=Presets.Select(p=>p.Item1).ToArray();panel.Children.Add(shortcuts);panel.Children.Add(Text("Sensitivity · higher needs a lighter movement"));panel.Children.Add(sensitivity);panel.Children.Add(reverse);
        panel.Children.Add(Text("Page mode: reverse only the assigned page axis, then release between pushes. Ring Scroll: reversing either direction updates both together."));
        panel.Children.Add(Button("Set pad up/down by page",()=>{if(Profile==null||Module?.Kind!=24)return;engine.CalibratingUid=Module.Uid;engine.CalibrationProfile=Profile;SetStatus("Push the pad UP once, then release it.");}));
        panel.Children.Add(Button("Set ring to up/down scrolling",()=>{if(Profile==null||Module?.Kind!=24)return;foreach(var evt in new[]{"rotation_left","rotation_right"})Profile.Mappings[Profiles.Key(Module.Uid,evt)]=new(){Action="scroll",Invert=true};engine.Save();RefreshEditor();}));
        panel.Children.Add(Button("Set selected light to red",()=>SetLight("FF0000")));panel.Children.Add(Button("Set selected light to white",()=>SetLight("FFFFFF")));
        panel.Children.Add(Button("Reset Module…",async()=>{if(Profile==null||Module==null)return;if(await Confirm("Clear this module’s assignments and colors in the selected profile? A backup is saved first.")){try{engine.Reset(Profile,Module);RefreshEditor();}catch(Exception ex){SetStatus(ex.Message);}}}));
        panel.Children.Add(Button("Reconnect controller",engine.Reconnect));
        var enabled=new CheckBox{Content="Enable assigned actions",IsChecked=engine.Settings.ActionsEnabled};enabled.IsCheckedChanged+=(_,_)=>{engine.Settings.ActionsEnabled=enabled.IsChecked==true;engine.Save();};panel.Children.Add(enabled);
        var startup=new CheckBox{Content="Start MONOcc when I sign in",IsChecked=Startup.Enabled};startup.IsCheckedChanged+=(_,_)=>{try{Startup.Enabled=startup.IsChecked==true;}catch(Exception ex){SetStatus(ex.Message);}};panel.Children.Add(startup);
        panel.Children.Add(Button("Export Linux backup…",async()=>{var file=await StorageProvider.SaveFilePickerAsync(new(){Title="Export MONOcc Linux backup",SuggestedFileName="MONOcc-Linux-Backup.json"});if(file!=null){await using var stream=await file.OpenWriteAsync();using var writer=new StreamWriter(stream);await writer.WriteAsync(System.Text.Json.JsonSerializer.Serialize(engine.Settings,Profiles.Json));}}));
        panel.Children.Add(Button("Restore Linux backup…",async()=>{var files=await StorageProvider.OpenFilePickerAsync(new(){Title="Choose MONOcc Linux backup",AllowMultiple=false});if(files.Count==0||!await Confirm("Replace settings with this Linux backup? Current settings are backed up first."))return;try{engine.Import(files[0].Path.LocalPath);RefreshProfiles();}catch(Exception ex){SetStatus(ex.Message);}}));
        panel.Children.Add(Text("USB access: the Debian package installs a rule for this Core model only. Reconnect USB after installing. Profiles are stored locally and remain after uninstall. This version does not control native Wayland applications. Some GNOME desktops hide tray icons; reopen MONOcc from the app launcher."));
        panel.Children.Add(Button("Open profiles & log folder",()=>{try{Process.Start(new ProcessStartInfo("xdg-open"){ArgumentList={engine.Store.Root},UseShellExecute=false});}catch(Exception ex){SetStatus(ex.Message);}}));
        panel.Children.Add(Button("Stop MONOcc and Exit",()=>((IClassicDesktopStyleApplicationLifetime)Application.Current!.ApplicationLifetime!).Shutdown()));
        apps.SelectionChanged+=(_,_)=>{if(updating)return;engine.Settings.SelectedProfile=Profile?.Id??"";engine.Save();RefreshEditor();};
        modules.SelectionChanged+=(_,_)=>RefreshControls();controls.SelectionChanged+=(_,_)=>RefreshEditor();
        actions.SelectionChanged+=(_,_)=>Edit(m=>m.Action=actions.SelectedIndex switch{1=>"shortcut",2=>"scroll",3=>"pages",_=>"none"});
        shortcuts.SelectionChanged+=(_,_)=>{if(shortcuts.SelectedIndex<0)return;var p=Presets[shortcuts.SelectedIndex];Edit(m=>{m.Action="shortcut";m.Shortcut=p.Item1;m.VirtualKey=p.Item2;m.Modifiers=p.Item3;});};
        sensitivity.ValueChanged+=(_,_)=>Edit(m=>m.Sensitivity=(double)(sensitivity.Value??1));
        reverse.IsCheckedChanged+=(_,_)=>{if(updating)return;if(Profile!=null&&Module!=null&&Selected?.Event is "rotation_left" or "rotation_right"&&Mapping.Action=="scroll"){foreach(var evt in new[]{"rotation_left","rotation_right"})if(Profile.Mappings.TryGetValue(Profiles.Key(Module.Uid,evt),out var m)&&m.Action=="scroll")m.Invert=reverse.IsChecked==true;engine.Save();}else Edit(m=>m.Invert=reverse.IsChecked==true);};
        engine.Changed+=Changed;Closed+=(_,_)=>engine.Changed-=Changed;RefreshProfiles();Changed();
    }
    void SetStatus(string message)=>status.Text=message;
    void Changed(){status.Text=engine.Status+" · "+(engine.Active?.Name??"No active profile")+"\n"+engine.Notice;string next=string.Join(';',engine.Modules);if(next!=signature){signature=next;modules.ItemsSource=engine.Modules.Select(m=>new Choice<Module>($"{m.Name} · {m.Index}",m)).ToList();modules.SelectedIndex=0;}}
    void RefreshProfiles(){updating=true;apps.ItemsSource=engine.Settings.Profiles.Select(p=>new Choice<Profile>(p.Name,p)).ToList();apps.SelectedIndex=Math.Max(0,engine.Settings.Profiles.FindIndex(p=>p.Id==engine.Settings.SelectedProfile));updating=false;RefreshEditor();}
    void RefreshControls(){controls.ItemsSource=Module?.Controls.Select(c=>new Choice<Monocc.Core.Control>(c.Title,c)).ToList();controls.SelectedIndex=0;RefreshEditor();}
    void RefreshEditor(){updating=true;actions.SelectedIndex=Mapping.Action switch{"shortcut"=>1,"scroll"=>2,"pages"=>3,_=>0};reverse.IsChecked=Mapping.Invert;sensitivity.Value=(decimal)Mapping.Sensitivity;shortcuts.SelectedIndex=Array.FindIndex(Presets,p=>p.Item2==Mapping.VirtualKey&&p.Item3==Mapping.Modifiers);updating=false;}
    void Edit(Action<Mapping> action){if(updating||Profile==null||Key==null)return;var m=Mapping;action(m);Profile.Mappings[Key]=m;if(m.Action=="pages"&&Module!=null)foreach(var other in Profile.Mappings.Keys.Where(k=>k!=Key&&k.StartsWith(Module.Uid+"|joystick_",StringComparison.Ordinal)&&Profile.Mappings[k].Action=="pages").ToArray())Profile.Mappings.Remove(other);engine.Save();RefreshEditor();}
    void SetLight(string color){if(Profile==null||Module==null)return;int sub=0;if(Selected!=null&&int.TryParse(Selected.Event.Split('_').Last(),out int n))sub=Math.Clamp(n-1,0,2);Profile.LedColors[Profiles.Key(Module.Uid,sub.ToString())]=color;engine.Save();}
    async Task<bool> Confirm(string message){bool yes=false;var window=new Window{Title="MONOcc confirmation",Width=470,Height=200,WindowStartupLocation=WindowStartupLocation.CenterOwner};var p=new StackPanel{Margin=new(20)};p.Children.Add(Text(message));p.Children.Add(Button("Continue",()=>{yes=true;window.Close();}));p.Children.Add(Button("Cancel",()=>window.Close()));window.Content=p;await window.ShowDialog(this);return yes;}
    sealed record Choice<T>(string Title,T Value){public override string ToString()=>Title;}
}
internal static class Startup {
    static string FilePath=>Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.UserProfile),".config","autostart","MONOcc.desktop");
    public static bool Enabled{get=>File.Exists(FilePath);set{if(value){Directory.CreateDirectory(Path.GetDirectoryName(FilePath)!);string path=Environment.ProcessPath!.Replace("\\","\\\\").Replace("\"","\\\"").Replace("`","\\`").Replace("$","\\$").Replace("%","%%");File.WriteAllText(FilePath,$"[Desktop Entry]\nType=Application\nName=MONOcc\nExec=\"{path}\" --background\nTryExec=/opt/monocc/MONOcc\nTerminal=false\nX-GNOME-Autostart-enabled=true\n");}else if(File.Exists(FilePath))File.Delete(FilePath);}}
}
