// SPDX-License-Identifier: GPL-3.0-or-later
using System.Runtime.InteropServices;
namespace Monocc.Linux;
internal sealed class Desktop:IDisposable {
    readonly IntPtr display;
    public bool Supported=>display!=IntPtr.Zero&&string.IsNullOrEmpty(Environment.GetEnvironmentVariable("WAYLAND_DISPLAY"));
    public string Session=>Supported?"X11 desktop control available":"This starting point requires an X11/Xorg desktop session. Wayland automation is not implemented.";
    public Desktop(){try{if(string.IsNullOrEmpty(Environment.GetEnvironmentVariable("WAYLAND_DISPLAY")))display=XOpenDisplay(IntPtr.Zero);}catch(DllNotFoundException){}}
    [DllImport("libX11.so.6")]static extern IntPtr XOpenDisplay(IntPtr name);
    [DllImport("libX11.so.6")]static extern int XCloseDisplay(IntPtr display);
    [DllImport("libX11.so.6")]static extern ulong XDefaultRootWindow(IntPtr display);
    [DllImport("libX11.so.6")]static extern ulong XInternAtom(IntPtr display,string name,int onlyIfExists);
    [DllImport("libX11.so.6")]static extern int XGetWindowProperty(IntPtr display,ulong window,ulong property,long offset,long length,int delete,ulong type,out ulong actual,out int format,out ulong count,out ulong after,out IntPtr data);
    [DllImport("libX11.so.6")]static extern int XFree(IntPtr data);
    [DllImport("libX11.so.6")]static extern byte XKeysymToKeycode(IntPtr display,ulong symbol);
    [DllImport("libX11.so.6")]static extern int XFlush(IntPtr display);
    [DllImport("libXtst.so.6")]static extern int XTestFakeKeyEvent(IntPtr display,uint code,int down,ulong delay);
    [DllImport("libXtst.so.6")]static extern int XTestFakeButtonEvent(IntPtr display,uint button,int down,ulong delay);
    ulong Property(ulong window,string name) {
        var atom=XInternAtom(display,name,1);if(atom==0)return 0;
        IntPtr data=IntPtr.Zero;
        try{int rc=XGetWindowProperty(display,window,atom,0,1,0,0,out _,out int format,out ulong count,out _,out data);return rc==0&&format==32&&count>0&&data!=IntPtr.Zero?unchecked((ulong)Marshal.ReadInt64(data)):0;}
        finally{if(data!=IntPtr.Zero)XFree(data);}
    }
    public string? ForegroundPath() {
        if(!Supported)return null;
        var window=Property(XDefaultRootWindow(display),"_NET_ACTIVE_WINDOW");if(window==0)return null;
        var pid=Property(window,"_NET_WM_PID");if(pid==0||pid==(ulong)Environment.ProcessId)return null;
        try{return File.ResolveLinkTarget($"/proc/{pid}/exe",true)?.FullName;}catch(IOException){return null;}catch(UnauthorizedAccessException){return null;}
    }
    static ulong Symbol(int vk)=>vk switch{0x21=>0xff55,0x22=>0xff56,13=>0xff0d,9=>0xff09,27=>0xff1b,0x25=>0xff51,0x26=>0xff52,0x27=>0xff53,0x28=>0xff54,>=112 and <=123=>(ulong)(0xffbe+vk-112),>=65 and <=90=>(ulong)(vk+32),_=>(ulong)vk};
    public bool Shortcut(int vk,int mods) {
        if(!Supported||vk<=0)return false;
        ulong[] modifier=[0xffe3,0xffe9,0xffe1,0xffeb];var down=new List<byte>();
        try {
            for(int n=0;n<4;n++)if((mods&(1<<n))!=0){byte code=XKeysymToKeycode(display,modifier[n]);if(code==0)return false;XTestFakeKeyEvent(display,code,1,0);down.Add(code);}
            byte key=XKeysymToKeycode(display,Symbol(vk));if(key==0)return false;XTestFakeKeyEvent(display,key,1,0);XTestFakeKeyEvent(display,key,0,0);return true;
        }catch(DllNotFoundException){return false;}finally{foreach(byte code in down.AsEnumerable().Reverse())XTestFakeKeyEvent(display,code,0,0);XFlush(display);}
    }
    public bool Scroll(double value) {
        if(!Supported)return false;
        try{uint button=value>0?4u:5u;for(int n=0;n<Math.Clamp((int)Math.Abs(value),1,12);n++){XTestFakeButtonEvent(display,button,1,0);XTestFakeButtonEvent(display,button,0,0);}XFlush(display);return true;}catch(DllNotFoundException){return false;}
    }
    public static string[] Ports() {
        var result=new List<string>();
        if(!Directory.Exists("/sys/class/tty"))return [];
        foreach(var path in Directory.EnumerateDirectories("/sys/class/tty").Where(p=>Path.GetFileName(p).StartsWith("ttyACM",StringComparison.Ordinal)||Path.GetFileName(p).StartsWith("ttyUSB",StringComparison.Ordinal)))try {
            var resolved=Directory.ResolveLinkTarget(Path.Combine(path,"device"),true);
            var device=resolved==null?null:new DirectoryInfo(resolved.FullName);
            for(var dir=device;dir!=null&&dir.FullName!="/sys";dir=dir.Parent) {
                var vendor=Path.Combine(dir.FullName,"idVendor");var product=Path.Combine(dir.FullName,"idProduct");
                if(File.Exists(vendor)&&File.Exists(product)&&File.ReadAllText(vendor).Trim().Equals("0483",StringComparison.OrdinalIgnoreCase)&&File.ReadAllText(product).Trim().Equals("a2c5",StringComparison.OrdinalIgnoreCase)){string port="/dev/"+Path.GetFileName(path);if(File.Exists(port))result.Add(port);break;}
            }
        }catch(IOException){}catch(UnauthorizedAccessException){}
        return result.ToArray();
    }
    public void Dispose(){if(display!=IntPtr.Zero)XCloseDisplay(display);}
}
