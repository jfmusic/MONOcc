// SPDX-License-Identifier: GPL-3.0-or-later
using System.Runtime.InteropServices;
using System.Text;
using Microsoft.Win32;
namespace Monocc.Windows;
internal static class Native {
    [DllImport("user32.dll")]internal static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")]static extern uint GetWindowThreadProcessId(IntPtr hwnd,out uint id);
    [DllImport("kernel32.dll")]static extern IntPtr OpenProcess(uint access,bool inherit,uint id);
    [DllImport("kernel32.dll",CharSet=CharSet.Unicode)]static extern bool QueryFullProcessImageName(IntPtr h,int flags,StringBuilder name,ref int size);
    [DllImport("kernel32.dll")]static extern bool CloseHandle(IntPtr h);
    [DllImport("user32.dll",SetLastError=true)]static extern uint SendInput(uint count,INPUT[] input,int size);
    [StructLayout(LayoutKind.Sequential)]struct INPUT {public uint type;public UNION data;}
    [StructLayout(LayoutKind.Explicit)]struct UNION {[FieldOffset(0)]public KEYBDINPUT key;[FieldOffset(0)]public MOUSEINPUT mouse;}
    [StructLayout(LayoutKind.Sequential)]struct KEYBDINPUT {public ushort vk,scan;public uint flags,time;public UIntPtr extra;}
    [StructLayout(LayoutKind.Sequential)]struct MOUSEINPUT {public int dx,dy;public uint data,flags,time;public UIntPtr extra;}
    internal static string? ForegroundPath() {
        var hwnd=GetForegroundWindow();if(hwnd==IntPtr.Zero)return null;GetWindowThreadProcessId(hwnd,out var id);if(id==Environment.ProcessId)return null;
        var h=OpenProcess(0x1000,false,id);if(h==IntPtr.Zero)return null;
        try{var text=new StringBuilder(32768);int size=text.Capacity;return QueryFullProcessImageName(h,0,text,ref size)?text.ToString():null;}finally{CloseHandle(h);}
    }
    static INPUT Key(int vk,bool up)=>new(){type=1,data=new(){key=new(){vk=(ushort)vk,flags=(uint)((up?2:0)|(vk is 0x21 or 0x22 or 0x25 or 0x26 or 0x27 or 0x28?1:0))}}};
    internal static bool ValidateInputStructure()=>Marshal.SizeOf<INPUT>()==40&&Marshal.OffsetOf<INPUT>("data").ToInt32()==8;
    internal static bool Shortcut(int vk,int mods) {
        if(vk is <=0 or >255)return false;
        int[] keys=[0x11,0x12,0x10,0x5b];var events=new List<INPUT>();
        for(int n=0;n<4;n++)if((mods&(1<<n))!=0)events.Add(Key(keys[n],false));
        events.Add(Key(vk,false));events.Add(Key(vk,true));
        for(int n=3;n>=0;n--)if((mods&(1<<n))!=0)events.Add(Key(keys[n],true));
        var sent=SendInput((uint)events.Count,events.ToArray(),Marshal.SizeOf<INPUT>());
        if(sent!=events.Count) {var release=keys.Select(k=>Key(k,true)).Append(Key(vk,true)).ToArray();SendInput((uint)release.Length,release,Marshal.SizeOf<INPUT>());return false;}return true;
    }
    internal static bool Scroll(int amount) {
        INPUT[] events=[new(){type=0,data=new(){mouse=new(){data=unchecked((uint)amount),flags=0x800}}}];
        return SendInput(1,events,Marshal.SizeOf<INPUT>())==1;
    }
    internal static string[] MonogramPorts() {
        var ports=new HashSet<string>(StringComparer.OrdinalIgnoreCase);
        using var usb=Registry.LocalMachine.OpenSubKey(@"SYSTEM\CurrentControlSet\Enum\USB");
        if(usb is null)return [];
        foreach(var device in usb.GetSubKeyNames().Where(n=>n.StartsWith("VID_0483&PID_A2C5",StringComparison.OrdinalIgnoreCase))) {
            using var instances=usb.OpenSubKey(device);if(instances is null)continue;
            foreach(var instance in instances.GetSubKeyNames()){using var parameters=instances.OpenSubKey(instance+@"\Device Parameters");if(parameters?.GetValue("PortName") is string port&&port.StartsWith("COM",StringComparison.OrdinalIgnoreCase))ports.Add(port);}
        }
        var present=new HashSet<string>(System.IO.Ports.SerialPort.GetPortNames(),StringComparer.OrdinalIgnoreCase);
        return ports.Where(present.Contains).Order().ToArray();
    }
}
