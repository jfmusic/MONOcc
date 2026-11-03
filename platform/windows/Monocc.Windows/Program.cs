// SPDX-License-Identifier: GPL-3.0-or-later
using Control = System.Windows.Forms.Control;
using System.IO.Pipes;
using System.Security.Principal;
using Monocc.Core;
namespace Monocc.Windows;
internal static class Program {
    internal static string UserKey=>WindowsIdentity.GetCurrent().User?.Value.Replace('-','_')??Environment.UserName;
    internal static string PipeName=>"MONOcc.Setup."+UserKey;
    [STAThread]static void Main(string[] args) {
        ApplicationConfiguration.Initialize();
        if(args.Contains("--smoke-test")) {
            string root=Path.Combine(Path.GetTempPath(),"MONOcc-Smoke-"+Guid.NewGuid().ToString("N"));
            try {
                if(!Native.ValidateInputStructure())throw new InvalidOperationException("Incorrect Windows input structure layout.");
                using var dispatcher=new Control();dispatcher.CreateControl();
                using var engine=new Engine(dispatcher,root);
                using(var window=new SetupWindow(engine)){window.Show();Application.DoEvents();if(window.ClientSize.Width<900)throw new InvalidOperationException("Setup window layout failed.");window.Close();Application.DoEvents();}
                Console.WriteLine("PASS: Windows input layout, setup creation, close and controller lifetime.");
                Environment.ExitCode=0;
            }catch(Exception e){Console.Error.WriteLine(e);Environment.ExitCode=1;}
            finally{if(Directory.Exists(root))Directory.Delete(root,true);}
            return;
        }
        try {
            if(args.Contains("--uninstall-cleanup")){Installer.Cleanup();return;}
            if(args.Contains("--uninstall")){Installer.Uninstall();return;}
            if(!args.Contains("--portable")&&!Installer.IsInstalled){Application.Run(new Installer());return;}
            using var mutex=new Mutex(true,@"Local\MONOcc.Controller."+UserKey,out bool first);
            if(!first){if(!args.Contains("--background"))try{using var client=new NamedPipeClientStream(".",PipeName,PipeDirection.Out);client.Connect(2000);client.WriteByte(1);}catch(IOException){}catch(TimeoutException){}return;}
            Application.Run(new BackgroundContext(!args.Contains("--background")));
            mutex.ReleaseMutex();
        }catch(Exception e){MessageBox.Show("MONOcc could not start.\n\n"+e.Message,"MONOcc",MessageBoxButtons.OK,MessageBoxIcon.Error);}
    }
}
internal sealed class BackgroundContext:ApplicationContext {
    readonly Control dispatch=new();readonly Engine engine;readonly NotifyIcon tray;readonly CancellationTokenSource cancel=new();SetupWindow? window;
    public BackgroundContext(bool showSetup) {
        dispatch.CreateControl();
        engine=new(dispatch,Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData),"MONOcc"));
        var menu=new ContextMenuStrip();menu.Items.Add("Open Setup…",null,(_,_)=>ShowSetup());
        var status=new ToolStripMenuItem("MONOcc starting"){Enabled=false};menu.Items.Insert(0,status);menu.Items.Add(new ToolStripSeparator());
        menu.Items.Add("Reconnect controller",null,(_,_)=>engine.Reconnect());
        var startup=new ToolStripMenuItem("Start at login"){Checked=Installer.StartAtLogin};startup.Click+=(_,_)=>{Installer.StartAtLogin=!Installer.StartAtLogin;startup.Checked=Installer.StartAtLogin;};menu.Items.Add(startup);
        menu.Items.Add(new ToolStripSeparator());menu.Items.Add("Stop MONOcc and Exit",null,(_,_)=>ExitThread());
        tray=new(){Icon=Icon.ExtractAssociatedIcon(Environment.ProcessPath!)??SystemIcons.Application,Text="MONOcc — background controller",ContextMenuStrip=menu,Visible=true};tray.DoubleClick+=(_,_)=>ShowSetup();
        engine.Changed+=()=>{status.Text=engine.Status;tray.Text=("MONOcc · "+engine.Status)[..Math.Min(63,("MONOcc · "+engine.Status).Length)];};
        _=Listen();if(showSetup)ShowSetup();engine.Log("MONOcc Windows 0.2.0 preview started.");
    }
    public void ShowSetup(){if(window==null||window.IsDisposed)window=new(engine);window.Show();window.WindowState=FormWindowState.Normal;window.Activate();}
    async Task Listen() {
        while(!cancel.IsCancellationRequested)try {
            using var pipe=new NamedPipeServerStream(Program.PipeName,PipeDirection.In,1,PipeTransmissionMode.Byte,PipeOptions.Asynchronous|PipeOptions.CurrentUserOnly);
            await pipe.WaitForConnectionAsync(cancel.Token);var buffer=new byte[1];if(await pipe.ReadAsync(buffer,cancel.Token)>0)dispatch.BeginInvoke(ShowSetup);
        }catch(OperationCanceledException){break;}catch(IOException){if(!cancel.IsCancellationRequested)await Task.Delay(500,cancel.Token);}
    }
    protected override void ExitThreadCore(){cancel.Cancel();window?.Dispose();engine.Dispose();tray.Visible=false;tray.Dispose();dispatch.Dispose();base.ExitThreadCore();}
}
