// SPDX-License-Identifier: GPL-3.0-or-later
using System.Diagnostics;
using System.Reflection;
using Microsoft.Win32;
namespace Monocc.Windows;
internal sealed class Installer:Form {
    internal static string InstallDir=>Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData),"Programs","MONOcc");
    internal static string InstalledExe=>Path.Combine(InstallDir,"MONOcc.exe");
    internal static bool IsInstalled=>string.Equals(Path.GetFullPath(Environment.ProcessPath!),InstalledExe,StringComparison.OrdinalIgnoreCase);
    const string UninstallKey=@"Software\Microsoft\Windows\CurrentVersion\Uninstall\MONOcc";
    const string RunKey=@"Software\Microsoft\Windows\CurrentVersion\Run";
    internal static bool StartAtLogin {
        get{using var key=Registry.CurrentUser.OpenSubKey(RunKey);return key?.GetValue("MONOcc") is string command&&command.StartsWith('"'+InstalledExe+'"',StringComparison.OrdinalIgnoreCase);}
        set{using var key=Registry.CurrentUser.CreateSubKey(RunKey);if(value){if(!File.Exists(InstalledExe))throw new IOException("Install MONOcc before enabling login startup.");key.SetValue("MONOcc",'"'+InstalledExe+"\" --background");}else key.DeleteValue("MONOcc",false);}
    }
    public Installer() {
        Text="MONOcc Setup · Windows preview";Size=new(590,470);StartPosition=FormStartPosition.CenterScreen;FormBorderStyle=FormBorderStyle.FixedDialog;MaximizeBox=false;Font=new("Segoe UI",10);
        var layout=new FlowLayoutPanel{Dock=DockStyle.Fill,FlowDirection=FlowDirection.TopDown,WrapContents=false,Padding=new(25),AutoScroll=true};Controls.Add(layout);
        layout.Controls.Add(new Label{Text="MONOcc for Windows",Font=new("Segoe UI",22,FontStyle.Bold),AutoSize=true});
        layout.Controls.Add(new Label{Text="Set up your controls once. Let MONOcc follow your apps in the background.",Width=510,Height=50});
        layout.Controls.Add(new Label{Text="Installs for your Windows account. No administrator access or separate .NET installation is needed. Your profiles stay on this computer.",Width=510,Height=65});
        var startup=new CheckBox{Text="Start MONOcc when I sign in",Checked=true,AutoSize=true};layout.Controls.Add(startup);
        layout.Controls.Add(new Label{Text="Preview release: Windows hardware support still needs testing. This build is unsigned. Do not disable Windows security protections to install it.",Width=510,Height=65,ForeColor=Color.DarkGoldenrod});
        var license=new Button{Text="View license",AutoSize=true};license.Click+=(_,_)=>ShowLicense();layout.Controls.Add(license);
        var install=new Button{Text="Install MONOcc",Width=170,Height=40};layout.Controls.Add(install);
        install.Click+=(_,_)=>{try{install.Enabled=false;Install(startup.Checked);Process.Start(new ProcessStartInfo(InstalledExe){UseShellExecute=true});Close();}catch(Exception e){install.Enabled=true;MessageBox.Show("Installation could not finish. Close MONOcc from its tray menu, then try again.\n\n"+e.Message,"MONOcc Setup");}};
    }
    static void Install(bool startup) {
        Directory.CreateDirectory(InstallDir);
        var source=Environment.ProcessPath!;var stage=Path.Combine(InstallDir,"MONOcc.installing");
        File.Copy(source,stage,true);File.Move(stage,InstalledExe,true);
        using(var key=Registry.CurrentUser.CreateSubKey(UninstallKey)) {
            key.SetValue("DisplayName","MONOcc");key.SetValue("DisplayVersion","0.2.0 preview");key.SetValue("InstallLocation",InstallDir);key.SetValue("DisplayIcon",InstalledExe);
            key.SetValue("UninstallString",'"'+InstalledExe+"\" --uninstall");key.SetValue("NoModify",1,RegistryValueKind.DWord);key.SetValue("NoRepair",1,RegistryValueKind.DWord);
        }
        var menu=Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.Programs),"MONOcc");Directory.CreateDirectory(menu);
        Shortcut(Path.Combine(menu,"MONOcc Setup.lnk"),InstalledExe,"");Shortcut(Path.Combine(menu,"Uninstall MONOcc.lnk"),InstalledExe,"--uninstall");
        StartAtLogin=startup;
    }
    static void Shortcut(string path,string target,string arguments) {
        object? shell=null;object? shortcut=null;
        try{shell=Activator.CreateInstance(Type.GetTypeFromProgID("WScript.Shell")!);dynamic s=shell!;shortcut=s.CreateShortcut(path);dynamic link=shortcut;link.TargetPath=target;link.Arguments=arguments;link.WorkingDirectory=InstallDir;link.IconLocation=InstalledExe+",0";link.Save();}
        finally{if(shortcut!=null)System.Runtime.InteropServices.Marshal.ReleaseComObject(shortcut);if(shell!=null)System.Runtime.InteropServices.Marshal.ReleaseComObject(shell);}
    }
    internal static void ShowLicense() {
        var name=Assembly.GetExecutingAssembly().GetManifestResourceNames().FirstOrDefault(n=>n.EndsWith("LICENSE",StringComparison.OrdinalIgnoreCase)||n.EndsWith("LICENSE.txt",StringComparison.OrdinalIgnoreCase));
        string text="GNU General Public License version 3. Source code is supplied with this preview distribution.";
        if(name!=null){using var stream=Assembly.GetExecutingAssembly().GetManifestResourceStream(name)!;using var reader=new StreamReader(stream);text=reader.ReadToEnd();}
        using var form=new Form{Text="MONOcc · GNU GPL v3",Size=new(760,600),StartPosition=FormStartPosition.CenterParent};form.Controls.Add(new TextBox{Multiline=true,ReadOnly=true,ScrollBars=ScrollBars.Vertical,Dock=DockStyle.Fill,Text=text});form.ShowDialog();
    }
    internal static void Uninstall() {
        if(!IsInstalled){MessageBox.Show("Open Uninstall MONOcc from the Start menu.","MONOcc");return;}
        if(MessageBox.Show("Remove MONOcc? Saved profiles and backups will be kept. Close MONOcc from its tray menu first.","Uninstall MONOcc",MessageBoxButtons.OKCancel)!=DialogResult.OK)return;
        string temp=Path.Combine(Path.GetTempPath(),"MONOcc-Uninstall-"+Guid.NewGuid().ToString("N")+".exe");File.Copy(InstalledExe,temp);
        Process.Start(new ProcessStartInfo(temp,"--uninstall-cleanup"){UseShellExecute=true});
    }
    internal static void Cleanup() {
        Thread.Sleep(1000);
        try {
            using var mutex=new Mutex(true,@"Local\MONOcc.Controller."+Program.UserKey,out bool free);
            if(!free){MessageBox.Show("MONOcc is still running. Choose Stop MONOcc and Exit from its tray menu, then uninstall again.","MONOcc");return;}
            for(int n=0;n<10;n++){try{if(File.Exists(InstalledExe))File.Delete(InstalledExe);break;}catch(IOException){Thread.Sleep(500);}}
            if(File.Exists(InstalledExe))throw new IOException("MONOcc.exe is still in use. Close it and try uninstalling again.");
            StartAtLogin=false;Registry.CurrentUser.DeleteSubKeyTree(UninstallKey,false);
            string shortcutDir=Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.Programs),"MONOcc");
            foreach(string file in new[]{"MONOcc Setup.lnk","Uninstall MONOcc.lnk"}){string path=Path.Combine(shortcutDir,file);if(File.Exists(path))File.Delete(path);}
            if(Directory.Exists(shortcutDir)&&!Directory.EnumerateFileSystemEntries(shortcutDir).Any())Directory.Delete(shortcutDir);
            if(Directory.Exists(InstallDir)&&!Directory.EnumerateFileSystemEntries(InstallDir).Any())Directory.Delete(InstallDir);
            mutex.ReleaseMutex();MessageBox.Show("MONOcc removed. Your profiles and backups were kept.","MONOcc");
        }catch(Exception e){MessageBox.Show("Could not finish uninstalling: "+e.Message,"MONOcc");}
    }
}
