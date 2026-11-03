// SPDX-License-Identifier: GPL-3.0-or-later
using System.Diagnostics;
using System.Drawing.Imaging;
using Monocc.Core;
namespace Monocc.Windows;
internal sealed class SetupWindow:Form {
    readonly Engine engine;
    readonly ListBox apps=new(){Dock=DockStyle.Fill,DisplayMember="Name"};
    readonly ListBox modules=new(){Dock=DockStyle.Fill,DisplayMember="Name"};
    readonly ListBox controls=new(){Dock=DockStyle.Fill,DisplayMember="Title"};
    readonly Label status=new(){Dock=DockStyle.Bottom,Height=64,Padding=new(14),AutoEllipsis=true};
    readonly Label heading=new(){AutoSize=true,Font=new("Segoe UI",18,FontStyle.Bold)};
    readonly FlowLayoutPanel editor=Column();
    readonly ComboBox action=new(){DropDownStyle=ComboBoxStyle.DropDownList,Width=280};
    readonly TextBox label=new(){Width=300,PlaceholderText="Optional action label"};
    readonly Label shortcut=new(){AutoSize=true};
    readonly NumericUpDown sensitivity=new(){DecimalPlaces=2,Minimum=.25m,Maximum=4,Increment=.25m,Value=1,Width=110};
    readonly CheckBox sliderLabels=new(){Text="Swap slider label order",AutoSize=true};
    readonly CheckBox reverse=new(){Text="Reverse direction",AutoSize=true};
    readonly Label reverseHelp=new(){Width=350,Height=60};
    readonly Button record=new(){Text="Record shortcut",AutoSize=true};
    readonly Button calibrate=new(){Text="Set pad up/down by page",AutoSize=true};
    readonly PictureBox preview=new(){Width=120,Height=120,SizeMode=PictureBoxSizeMode.Zoom};
    readonly ComboBox display=new(){DropDownStyle=ComboBoxStyle.DropDownList,Width=160};
    readonly CheckBox startup=new(){Text="Start MONOcc when I sign in",AutoSize=true};
    bool updating,recording;
    string moduleSignature="";
    Profile? Profile=>apps.SelectedItem as Profile;
    Module? Module=>modules.SelectedItem as Module;
    Monocc.Core.Control? SelectedControl=>controls.SelectedItem as Monocc.Core.Control;
    string? Key=>Module is {} m&&SelectedControl is {} c?Profiles.Key(m.Uid,c.Event):null;
    Mapping Mapping=>Key is {} key&&Profile?.Mappings.TryGetValue(key,out var m)==true?m:new();
    static FlowLayoutPanel Column()=>new(){Dock=DockStyle.Fill,FlowDirection=FlowDirection.TopDown,WrapContents=false,AutoScroll=true,Padding=new(12)};
    static Button Button(string text,Action handler){var b=new Button{Text=text,AutoSize=true,MinimumSize=new(100,32)};b.Click+=(_,_)=>handler();return b;}
    static Label TextLine(string text,int height=44)=>new(){Text=text,Width=355,Height=height};
    public SetupWindow(Engine engine) {
        this.engine=engine;Text="MONOcc v0.2.0 · Windows preview";Size=new(1180,850);MinimumSize=new(1000,750);StartPosition=FormStartPosition.CenterScreen;Font=new("Segoe UI",10);KeyPreview=true;
        var tabs=new TabControl{Dock=DockStyle.Fill};var setup=new TabPage("Your apps & controls");var help=new TabPage("Settings & Help");tabs.TabPages.Add(setup);tabs.TabPages.Add(help);Controls.Add(tabs);Controls.Add(status);
        var root=new TableLayoutPanel{Dock=DockStyle.Fill,ColumnCount=3,RowCount=1};root.ColumnStyles.Add(new(SizeType.Absolute,240));root.ColumnStyles.Add(new(SizeType.Absolute,260));root.ColumnStyles.Add(new(SizeType.Percent,100));setup.Controls.Add(root);
        var left=new TableLayoutPanel{Dock=DockStyle.Fill,RowCount=3,ColumnCount=1};left.RowStyles.Add(new(SizeType.Absolute,70));left.RowStyles.Add(new(SizeType.Percent,100));left.RowStyles.Add(new(SizeType.Absolute,190));root.Controls.Add(left,0,0);
        left.Controls.Add(new Label{Text="MONOcc\nv0.2.0 · Your apps",Dock=DockStyle.Fill,Font=new("Segoe UI",16,FontStyle.Bold),Padding=new(10)},0,0);left.Controls.Add(apps,0,1);
        var appButtons=Column();appButtons.Controls.Add(Button("Use last active app",AddForeground));appButtons.Controls.Add(Button("Choose an app…",AddApp));appButtons.Controls.Add(Button("Duplicate profile",Duplicate));appButtons.Controls.Add(Button("Rename profile",Rename));appButtons.Controls.Add(Button("Remove profile",Remove));left.Controls.Add(appButtons,0,2);
        var middle=new TableLayoutPanel{Dock=DockStyle.Fill,RowCount=5,ColumnCount=1};middle.RowStyles.Add(new(SizeType.Absolute,35));middle.RowStyles.Add(new(SizeType.Percent,35));middle.RowStyles.Add(new(SizeType.Absolute,40));middle.RowStyles.Add(new(SizeType.Percent,65));middle.RowStyles.Add(new(SizeType.Absolute,75));root.Controls.Add(middle,1,0);
        middle.Controls.Add(new Label{Text="Connected units",Dock=DockStyle.Fill,Padding=new(8)},0,0);middle.Controls.Add(modules,0,1);
        middle.Controls.Add(Button("Touch a control to select",()=>{engine.Learning=true;Tell("Move or press one control to select it.");}),0,2);middle.Controls.Add(controls,0,3);
        var moduleButtons=new FlowLayoutPanel{Dock=DockStyle.Fill};moduleButtons.Controls.Add(Button("Name unit",RenameUnit));moduleButtons.Controls.Add(Button("Reset Module…",Reset));middle.Controls.Add(moduleButtons,0,4);
        root.Controls.Add(editor,2,0);editor.Controls.Add(heading);editor.Controls.Add(TextLine("1. Connect your Core.  2. Add an app.\n3. Assign a control.  4. Close setup to keep it running.",52));
        editor.Controls.Add(preview);display.Items.AddRange(new object[]{"Icon + name","Name","Icon"});editor.Controls.Add(display);editor.Controls.Add(Button("Customize Program Icon…",CustomIcon));editor.Controls.Add(Button("Use original program icon",()=>{if(Profile is {} p){p.IconPng=null;engine.Save();RefreshEditor();}}));
        editor.Controls.Add(new Label{Text="Console rotation",AutoSize=true});var rotation=new ComboBox{DropDownStyle=ComboBoxStyle.DropDownList,Width=180};rotation.Items.AddRange(new object[]{"0°","90°","180°","270°"});rotation.SelectedIndex=engine.Settings.Orientation/90;rotation.SelectedIndexChanged+=(_,_)=>{engine.Settings.Orientation=rotation.SelectedIndex*90;engine.Save();RefreshEditor();};editor.Controls.Add(rotation);
        editor.Controls.Add(new Label{Text="Selected control",AutoSize=true,Font=new("Segoe UI",12,FontStyle.Bold)});editor.Controls.Add(label);editor.Controls.Add(sliderLabels);sliderLabels.CheckedChanged+=(_,_)=>{if(updating||Module?.Kind!=10)return;engine.Settings.SliderFlipped[Module.Uid]=sliderLabels.Checked;engine.Save();RefreshControls();};
        action.Items.AddRange(new object[]{"Unassigned","Keyboard shortcut","Scroll","Page up / down","Output volume (slider)"});editor.Controls.Add(action);editor.Controls.Add(shortcut);editor.Controls.Add(record);editor.Controls.Add(calibrate);
        editor.Controls.Add(Button("Set ring to up/down scrolling",SetRing));editor.Controls.Add(new Label{Text="Sensitivity · higher needs a lighter movement",AutoSize=true});editor.Controls.Add(sensitivity);editor.Controls.Add(reverse);editor.Controls.Add(reverseHelp);
        editor.Controls.Add(Button("Choose saved light color…",ChooseColor));
        var lights=new ComboBox{DropDownStyle=ComboBoxStyle.DropDownList,Width=220};lights.Items.AddRange(new object[]{"Follow active app","All off","All white","All red","All green","All blue"});lights.SelectedIndex=0;lights.SelectedIndexChanged+=(_,_)=>engine.TestLights(lights.SelectedIndex switch{1=>"000000",2=>"FFFFFF",3=>"FF0000",4=>"00FF00",5=>"0000FF",_=>null});editor.Controls.Add(lights);
        BuildHelp(help);
        apps.SelectedIndexChanged+=(_,_)=>{if(updating)return;engine.Settings.SelectedProfile=Profile?.Id??"";engine.Save();RefreshEditor();};
        modules.FormattingEnabled=true;modules.Format+=(_,e)=>{if(e.ListItem is Module m)e.Value=engine.Settings.UnitNames.GetValueOrDefault(m.Uid,$"{m.Name} · Unit {m.Index}");};
        modules.SelectedIndexChanged+=(_,_)=>RefreshControls();controls.SelectedIndexChanged+=(_,_)=>RefreshEditor();
        label.Leave+=(_,_)=>Edit(m=>m.Label=label.Text);
        action.SelectedIndexChanged+=(_,_)=>Edit(m=>m.Action=action.Text switch{"Keyboard shortcut"=>"shortcut","Scroll"=>"scroll","Page up / down"=>"pages","Output volume (slider)"=>"volume",_=>"none"});
        sensitivity.ValueChanged+=(_,_)=>Edit(m=>m.Sensitivity=(double)sensitivity.Value);
        reverse.CheckedChanged+=(_,_)=>{
            if(updating||Profile==null||Module==null||SelectedControl==null)return;
            if(SelectedControl.Event is "rotation_left" or "rotation_right"&&Mapping.Action=="scroll") {foreach(var evt in new[]{"rotation_left","rotation_right"})if(Profile.Mappings.TryGetValue(Profiles.Key(Module.Uid,evt),out var m)&&m.Action=="scroll")m.Invert=reverse.Checked;engine.Save();RefreshEditor();}
            else Edit(m=>m.Invert=reverse.Checked);
        };
        display.SelectedIndexChanged+=(_,_)=>{if(updating||Profile==null)return;Profile.Display=display.SelectedIndex switch{1=>"name",2=>"icon",_=>"both"};engine.Save();RefreshEditor();};
        record.Click+=(_,_)=>{if(Key==null)return;recording=true;record.Text="Press shortcut… Esc cancels";record.Focus();};
        calibrate.Click+=(_,_)=>{if(Profile==null||Module?.Kind!=24)return;engine.CalibratingUid=Module.Uid;engine.CalibrationProfile=Profile;Tell("Push the pad UP once, then release it. Close setup to try it in your app.");};
        KeyDown+=RecordKey;
        engine.Changed+=UpdateStatus;engine.ControlTouched+=Touched;
        FormClosed+=(_,_)=>{engine.Changed-=UpdateStatus;engine.ControlTouched-=Touched;engine.FinishSetup();preview.Image?.Dispose();};
        RefreshProfiles();RefreshModules();UpdateStatus();
    }
    void Tell(string text)=>MessageBox.Show(this,text,"MONOcc",MessageBoxButtons.OK,MessageBoxIcon.Information);
    void Error(Exception e)=>MessageBox.Show(this,e.Message,"MONOcc",MessageBoxButtons.OK,MessageBoxIcon.Warning);
    void Edit(Action<Mapping> change) {
        if(updating||Profile==null||Key==null)return;var m=Mapping;change(m);Profile.Mappings[Key]=m;
        if(m.Action=="pages"&&Module!=null)foreach(var other in Profile.Mappings.Keys.Where(k=>k.StartsWith(Module.Uid+"|joystick_",StringComparison.Ordinal)&&k!=Key&&Profile.Mappings[k].Action=="pages").ToArray())Profile.Mappings.Remove(other);
        engine.Save();RefreshEditor();
    }
    void UpdateStatus() {
        if(IsDisposed)return;status.Text=engine.Status+" · "+(engine.Active?.Name??"No profile for the active app")+"\n"+engine.Notice;
        string signature=string.Join(';',engine.Modules.Select(m=>$"{m.Uid}:{m.Index}:{m.Kind}"));if(signature!=moduleSignature){moduleSignature=signature;RefreshModules();}
    }
    void RefreshProfiles(){var id=Profile?.Id??engine.Settings.SelectedProfile;updating=true;apps.Items.Clear();foreach(var p in engine.Settings.Profiles)apps.Items.Add(p);apps.SelectedItem=engine.Settings.Profiles.FirstOrDefault(p=>p.Id==id)??engine.Settings.Profiles.FirstOrDefault();updating=false;RefreshEditor();}
    void RefreshModules(){var uid=Module?.Uid;modules.Items.Clear();foreach(var m in engine.Modules)modules.Items.Add(m);modules.SelectedItem=engine.Modules.FirstOrDefault(m=>m.Uid==uid)??engine.Modules.FirstOrDefault();RefreshControls();}
    void RefreshControls(){var evt=SelectedControl?.Event;controls.Items.Clear();if(Module is {} m){var items=m.Controls;if(m.Kind==10&&engine.Settings.SliderFlipped.GetValueOrDefault(m.Uid,true))items=items.Reverse();foreach(var c in items)controls.Items.Add(c);}controls.SelectedItem=controls.Items.Cast<Monocc.Core.Control>().FirstOrDefault(c=>c.Event==evt)??controls.Items.Cast<Monocc.Core.Control>().FirstOrDefault();RefreshEditor();}
    void RefreshEditor() {
        updating=true;heading.Text=Profile?.Name??"Add your first app";
        label.Enabled=action.Enabled=record.Enabled=reverse.Enabled=sensitivity.Enabled=Key!=null&&Profile!=null;
        label.Text=Mapping.Label;action.Items.Clear();action.Items.AddRange(new object[]{"Unassigned","Keyboard shortcut"});if(SelectedControl?.Event.StartsWith("pressed_",StringComparison.Ordinal)!=true)action.Items.Add("Scroll");if(Module?.Kind==24&&SelectedControl?.Event.StartsWith("joystick_",StringComparison.Ordinal)==true)action.Items.Add("Page up / down");if(Module?.Kind==10)action.Items.Add("Output volume (slider)");action.SelectedItem=Mapping.Action switch{"shortcut"=>"Keyboard shortcut","scroll"=>"Scroll","pages"=>"Page up / down","volume"=>"Output volume (slider)",_=>"Unassigned"};shortcut.Text=Mapping.Shortcut.Length==0?"No shortcut recorded":Mapping.Shortcut;
        sensitivity.Value=(decimal)Math.Clamp(Mapping.Sensitivity,.25,4);reverse.Checked=Mapping.Invert;
        sliderLabels.Visible=Module?.Kind==10;sliderLabels.Checked=Module is {} selectedModule&&engine.Settings.SliderFlipped.GetValueOrDefault(selectedModule.Uid,true);
        display.SelectedIndex=Profile?.Display switch{"name"=>1,"icon"=>2,_=>0};calibrate.Enabled=Module?.Kind==24&&Profile!=null;
        reverseHelp.Text=Mapping.Action=="pages"?"Reverse only this assigned page axis. Leave the other pad axis unchanged. Release between pushes; there is no timed delay.":SelectedControl?.Event is "rotation_left" or "rotation_right"?"When both ring directions use Scroll, reversing either updates both together.":"Reverse changes the direction of this control.";
        var old=preview.Image;preview.Image=Engine.ScreenImage(Profile,Profile?.AppPath??"",engine.Settings.Orientation);old?.Dispose();updating=false;
    }
    void AddPath(string path) {
        if(!File.Exists(path)||!path.EndsWith(".exe",StringComparison.OrdinalIgnoreCase)){Tell("Choose an installed Windows application (.exe).");return;}
        var existing=engine.Settings.Profiles.FirstOrDefault(p=>string.Equals(p.AppPath,path,StringComparison.OrdinalIgnoreCase));
        if(existing==null){existing=new(){Name=Path.GetFileNameWithoutExtension(path),AppPath=Path.GetFullPath(path)};engine.Settings.Profiles.Add(existing);}
        engine.Settings.SelectedProfile=existing.Id;engine.Save();apps.SelectedItem=null;RefreshProfiles();apps.SelectedItem=existing;
    }
    void AddForeground(){if(string.IsNullOrEmpty(engine.Foreground)){Tell("Switch to the app you want to control, then return to MONOcc and choose Use last active app.");return;}AddPath(engine.Foreground);}
    void AddApp(){using var dialog=new OpenFileDialog{Title="Choose the application to control",Filter="Windows applications (*.exe)|*.exe",CheckFileExists=true};if(dialog.ShowDialog(this)==DialogResult.OK)AddPath(dialog.FileName);}
    void Duplicate(){if(Profile is not {} p)return;using var dialog=new OpenFileDialog{Title="Choose the application for the copied profile",Filter="Windows applications (*.exe)|*.exe",CheckFileExists=true};if(dialog.ShowDialog(this)!=DialogResult.OK)return;var path=dialog.FileName;if(engine.Settings.Profiles.Any(x=>string.Equals(x.AppPath,path,StringComparison.OrdinalIgnoreCase))){Tell("A profile already exists for this application. Choose it from the app list to edit it.");return;}var clone=System.Text.Json.JsonSerializer.Deserialize<Profile>(System.Text.Json.JsonSerializer.Serialize(p,Profiles.Json),Profiles.Json)!;clone.Id=Guid.NewGuid().ToString();clone.Name=Path.GetFileNameWithoutExtension(path);clone.AppPath=path;engine.Settings.Profiles.Add(clone);engine.Save();RefreshProfiles();apps.SelectedItem=clone;}
    void Rename(){if(Profile is not {} p)return;var name=Prompt("Profile name",p.Name);if(!string.IsNullOrWhiteSpace(name)){p.Name=name.Trim();engine.Save();RefreshProfiles();}}
    void Remove(){if(Profile is not {} p||MessageBox.Show(this,$"Remove the {p.Name} profile? A backup is saved first.","MONOcc",MessageBoxButtons.OKCancel)!=DialogResult.OK)return;try{engine.Store.Backup(engine.Settings,"profile removal");engine.Settings.Profiles.Remove(p);engine.Save();RefreshProfiles();}catch(Exception e){Error(e);}}
    void RenameUnit(){if(Module is not {} m)return;var name=Prompt("Unit name",engine.Settings.UnitNames.GetValueOrDefault(m.Uid,m.Name));if(!string.IsNullOrWhiteSpace(name)){engine.Settings.UnitNames[m.Uid]=name.Trim();engine.Save();RefreshModules();}}
    void Reset(){if(Profile is not {} p||Module is not {} m)return;if(MessageBox.Show(this,$"Clear assignments and colors for {m.Name} in {p.Name}? Other modules and profiles are kept. A backup is saved first.","Reset Module",MessageBoxButtons.OKCancel,MessageBoxIcon.Warning)!=DialogResult.OK)return;try{engine.Reset(p,m);RefreshEditor();}catch(Exception e){Error(e);}}
    void Touched(Module module,string evt){modules.SelectedItem=module;controls.SelectedItem=controls.Items.Cast<Monocc.Core.Control>().FirstOrDefault(c=>c.Event==evt);RefreshEditor();}
    void SetRing(){if(Module?.Kind!=24||Profile==null)return;foreach(var evt in new[]{"rotation_left","rotation_right"})Profile.Mappings[Profiles.Key(Module.Uid,evt)]=new(){Action="scroll",Invert=true};engine.Save();RefreshEditor();}
    void ChooseColor(){if(Profile==null||Module==null)return;using var dialog=new ColorDialog{FullOpen=true};if(dialog.ShowDialog(this)!=DialogResult.OK)return;int sub=0;if(SelectedControl!=null&&int.TryParse(SelectedControl.Event.Split('_').Last(),out int n))sub=Math.Clamp(n-1,0,2);Profile.LedColors[Profiles.Key(Module.Uid,sub.ToString())]=$"{dialog.Color.R:X2}{dialog.Color.G:X2}{dialog.Color.B:X2}";engine.Save();}
    void CustomIcon(){if(Profile==null)return;using var dialog=new OpenFileDialog{Title="Choose Core screen image",Filter="Images|*.png;*.jpg;*.jpeg;*.bmp",CheckFileExists=true};if(dialog.ShowDialog(this)!=DialogResult.OK)return;try{if(new FileInfo(dialog.FileName).Length>20_000_000)throw new IOException("Choose an image smaller than 20 MB.");using var image=Image.FromFile(dialog.FileName);using var resized=new Bitmap(240,240);using(var g=Graphics.FromImage(resized)){g.Clear(Color.Transparent);float ratio=Math.Min(240f/image.Width,240f/image.Height);g.DrawImage(image,(240-image.Width*ratio)/2,(240-image.Height*ratio)/2,image.Width*ratio,image.Height*ratio);}using var stream=new MemoryStream();resized.Save(stream,ImageFormat.Png);Profile.IconPng=Convert.ToBase64String(stream.ToArray());engine.Save();RefreshEditor();}catch(Exception e){Error(e);}}
    void RecordKey(object? sender,KeyEventArgs e) {
        if(!recording)return;e.SuppressKeyPress=true;e.Handled=true;if(e.KeyCode is Keys.ControlKey or Keys.ShiftKey or Keys.Menu or Keys.LWin or Keys.RWin)return;
        recording=false;record.Text="Record shortcut";if(e.KeyCode==Keys.Escape)return;
        int mods=(e.Control?1:0)|(e.Alt?2:0)|(e.Shift?4:0);Edit(m=>{m.Action="shortcut";m.VirtualKey=(int)e.KeyCode;m.Modifiers=mods;m.Shortcut=(e.Control?"Ctrl+":"")+(e.Alt?"Alt+":"")+(e.Shift?"Shift+":"")+e.KeyCode;});
    }
    void BuildHelp(TabPage page) {
        var panel=Column();page.Controls.Add(panel);panel.Controls.Add(new Label{Text="Connection & background setup",AutoSize=true,Font=new("Segoe UI",16,FontStyle.Bold)});
        panel.Controls.Add(TextLine("Close this setup window to keep profiles running. Use the MONOcc tray icon (possibly under the ^ arrow) or Start menu to reopen it. Stop MONOcc and Exit stops the controller.",88));
        startup.Checked=Installer.StartAtLogin;startup.CheckedChanged+=(_,_)=>{try{Installer.StartAtLogin=startup.Checked;}catch(Exception e){Error(e);}};panel.Controls.Add(startup);
        var actions=new CheckBox{Text="Enable assigned actions",Checked=engine.Settings.ActionsEnabled,AutoSize=true};actions.CheckedChanged+=(_,_)=>{engine.Settings.ActionsEnabled=actions.Checked;engine.Save();};panel.Controls.Add(actions);
        panel.Controls.Add(Button("Reconnect controller",engine.Reconnect));panel.Controls.Add(Button("Disconnect controller",engine.Disconnect));
        panel.Controls.Add(TextLine("USB connection is automatic for a detected Monogram Core serial port. Close original Creator and other controller software first. This preview supports CoreV2, not legacy Palette Gear.",80));
        var port=new ComboBox{Width=220,DropDownStyle=ComboBoxStyle.DropDownList};port.Items.Add("Automatic · Monogram only");foreach(var p in System.IO.Ports.SerialPort.GetPortNames())port.Items.Add(p);port.SelectedIndex=0;port.SelectedIndexChanged+=(_,_)=>{engine.PortOverride=port.SelectedIndex==0?null:port.Text;engine.Reconnect();};panel.Controls.Add(port);
        panel.Controls.Add(TextLine("Choose a COM port manually only if you have identified it as your Monogram Core. MONOcc will send controller commands to the selected port.",60));
        panel.Controls.Add(new Label{Text="Permissions & troubleshooting",AutoSize=true,Font=new("Segoe UI",14,FontStyle.Bold)});
        panel.Controls.Add(TextLine("Windows does not use the Mac Accessibility permission. Keep MONOcc and the controlled app running normally. Windows can block shortcuts sent to an app running as administrator.",80));
        panel.Controls.Add(Button("Open Startup settings",()=>Process.Start(new ProcessStartInfo("ms-settings:startupapps"){UseShellExecute=true})));
        panel.Controls.Add(TextLine("If no controller appears, check Windows Device Manager for its USB/COM port. This preview does not replace USB drivers or change security settings. Driver compatibility still needs physical testing.",80));
        panel.Controls.Add(Button("Open Device Manager",()=>Process.Start(new ProcessStartInfo("devmgmt.msc"){UseShellExecute=true})));
        panel.Controls.Add(Button("Export backup…",()=>{using var dialog=new SaveFileDialog{Filter="MONOcc Windows backup (*.json)|*.json",FileName=$"MONOcc Windows Backup {DateTime.Now:yyyy-MM-dd}.json"};if(dialog.ShowDialog(this)==DialogResult.OK)try{File.WriteAllText(dialog.FileName,System.Text.Json.JsonSerializer.Serialize(engine.Settings,Profiles.Json));}catch(Exception e){Error(e);}}));
        panel.Controls.Add(Button("Restore backup…",()=>{using var dialog=new OpenFileDialog{Filter="MONOcc Windows backup (*.json)|*.json"};if(dialog.ShowDialog(this)!=DialogResult.OK)return;if(MessageBox.Show(this,"Replace your current settings with this backup? A copy of current settings is saved first.","MONOcc",MessageBoxButtons.OKCancel)!=DialogResult.OK)return;try{engine.Import(dialog.FileName);RefreshProfiles();RefreshModules();}catch(Exception e){Error(e);}}));
        panel.Controls.Add(TextLine("Mac keyboard codes differ from Windows. This preview accepts Windows backups only; create and check Windows shortcuts before sharing a profile.",60));
        panel.Controls.Add(Button("Open settings & backups folder",()=>Process.Start(new ProcessStartInfo("explorer.exe",engine.Store.Root){UseShellExecute=true})));
        panel.Controls.Add(Button("View license",Installer.ShowLicense));
    }
    string? Prompt(string title,string value){using var form=new Form{Text=title,Size=new(540,170),StartPosition=FormStartPosition.CenterParent,FormBorderStyle=FormBorderStyle.FixedDialog};var input=new TextBox{Text=value,Dock=DockStyle.Top};var ok=new Button{Text="Save",DialogResult=DialogResult.OK,Dock=DockStyle.Bottom};form.Controls.Add(input);form.Controls.Add(ok);form.AcceptButton=ok;return form.ShowDialog(this)==DialogResult.OK?input.Text:null;}
}
