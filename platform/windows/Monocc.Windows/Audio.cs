// SPDX-License-Identifier: GPL-3.0-or-later
using System.Runtime.InteropServices;
namespace Monocc.Windows;
// Windows Core Audio: default render endpoint, scalar master volume.
internal static class Audio {
    internal static void SetVolume(float value) {
        object? enumerator=null;IMMDevice? device=null;object? endpoint=null;
        try {
            enumerator=new MMDeviceEnumerator();((IMMDeviceEnumerator)enumerator).GetDefaultAudioEndpoint(0,0,out device);
            var iid=typeof(IAudioEndpointVolume).GUID;device.Activate(ref iid,23,IntPtr.Zero,out endpoint);
            var volume=(IAudioEndpointVolume)endpoint;Guid context=Guid.Empty;Marshal.ThrowExceptionForHR(volume.SetMasterVolumeLevelScalar(Math.Clamp(value,0,1),ref context));
        } finally {if(endpoint!=null)Marshal.ReleaseComObject(endpoint);if(device!=null)Marshal.ReleaseComObject(device);if(enumerator!=null)Marshal.ReleaseComObject(enumerator);}
    }
    [ComImport,Guid("BCDE0395-E52F-467C-8E3D-C4579291692E")]class MMDeviceEnumerator{}
    [ComImport,Guid("A95664D2-9614-4F35-A746-DE8DB63617E6"),InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]interface IMMDeviceEnumerator {
        void EnumAudioEndpoints(int flow,uint mask,out IntPtr devices);
        void GetDefaultAudioEndpoint(int flow,int role,out IMMDevice device);
    }
    [ComImport,Guid("D666063F-1587-4E43-81F1-B948E807363F"),InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]interface IMMDevice {
        void Activate(ref Guid iid,uint context,IntPtr parameters,[MarshalAs(UnmanagedType.IUnknown)]out object instance);
    }
    [ComImport,Guid("5CDF2C82-841E-4546-9722-0CF74078229A"),InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]interface IAudioEndpointVolume {
        [PreserveSig]int RegisterControlChangeNotify(IntPtr notify);
        [PreserveSig]int UnregisterControlChangeNotify(IntPtr notify);
        [PreserveSig]int GetChannelCount(out uint count);
        [PreserveSig]int SetMasterVolumeLevel(float level,ref Guid context);
        [PreserveSig]int SetMasterVolumeLevelScalar(float level,ref Guid context);
    }
}
