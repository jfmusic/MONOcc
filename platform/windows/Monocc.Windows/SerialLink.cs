// SPDX-License-Identifier: GPL-3.0-or-later
using System.IO.Ports;
using System.Text.Json.Nodes;
using System.Threading.Channels;
using Monocc.Core;
namespace Monocc.Windows;
internal sealed class SerialLink:IDisposable {
    readonly SerialPort port;readonly Framer framer=new();readonly Channel<byte[]> writes=Channel.CreateBounded<byte[]>(new BoundedChannelOptions(256){SingleReader=true,FullMode=BoundedChannelFullMode.Wait});
    public event Action<JsonNode>? Message;
    public event Action<string>? Failed;
    int closed;
    public string Port=>port.PortName;
    public SerialLink(string name) {
        port=new(name,300000,Parity.None,8,StopBits.One){ReadTimeout=200,WriteTimeout=1500,DtrEnable=false,RtsEnable=false,Handshake=Handshake.None};
        port.Open();
        _=Task.Run(Read);_=Task.Run(Write);
    }
    public void Send(byte[] bytes){if(closed==0&&!writes.Writer.TryWrite(bytes))Failed?.Invoke("Controller output queue is full. Reconnecting.");}
    void Read() {
        var buffer=new byte[4096];
        while(closed==0)try{int count=port.Read(buffer,0,buffer.Length);foreach(var node in framer.Feed(buffer[..count]))Message?.Invoke(node);}catch(TimeoutException){}catch(Exception e)when(e is IOException or InvalidOperationException or UnauthorizedAccessException){if(closed==0)Failed?.Invoke(e.Message);break;}
    }
    async Task Write(){try{await foreach(var bytes in writes.Reader.ReadAllAsync()){if(closed!=0)break;port.Write(bytes,0,bytes.Length);}}catch(Exception e)when(e is IOException or InvalidOperationException or UnauthorizedAccessException or TimeoutException){if(closed==0)Failed?.Invoke(e.Message);}}
    public void Dispose(){if(Interlocked.Exchange(ref closed,1)!=0)return;writes.Writer.TryComplete();try{port.Close();}catch(IOException){}port.Dispose();}
}
