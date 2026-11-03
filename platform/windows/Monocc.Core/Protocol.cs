// SPDX-License-Identifier: GPL-3.0-or-later
using System.Buffers.Binary;
using System.Text;
using System.Text.Json.Nodes;
namespace Monocc.Core;

// CoreV2 MessagePack transport, derived from the GPL sextant protocol implementation.
public static class Wire {
    public static byte[] Frame(object value) {
        using var body=new MemoryStream();Write(body,value);
        using var output=new MemoryStream();output.WriteByte(0x7e);
        foreach(var b in body.ToArray())if(b is 0x7d or 0x7e){output.WriteByte(0x7d);output.WriteByte((byte)(b^0x20));}else output.WriteByte(b);
        output.WriteByte(0x7e);return output.ToArray();
    }
    public static byte[] Command(string name,object value)=>Frame(new Dictionary<string,object>{{name,value}});
    static void Number(Stream s,long n) {
        if(n is >=0 and <=127){s.WriteByte((byte)n);return;}
        if(n is >=-32 and <0){s.WriteByte(unchecked((byte)n));return;}
        Span<byte>b=stackalloc byte[8];
        if(n>=0&&n<=byte.MaxValue){s.WriteByte(0xcc);s.WriteByte((byte)n);}
        else if(n>=0&&n<=ushort.MaxValue){s.WriteByte(0xcd);BinaryPrimitives.WriteUInt16BigEndian(b,(ushort)n);s.Write(b[..2]);}
        else if(n>=0&&n<=uint.MaxValue){s.WriteByte(0xce);BinaryPrimitives.WriteUInt32BigEndian(b,(uint)n);s.Write(b[..4]);}
        else{s.WriteByte(0xd3);BinaryPrimitives.WriteInt64BigEndian(b,n);s.Write(b);}
    }
    static void Length(Stream s,int n,int fix,int small,int large) {
        if(n<16&&fix!=0){s.WriteByte((byte)(fix+n));return;}
        if(small!=0&&n<=255){s.WriteByte((byte)small);s.WriteByte((byte)n);return;}
        s.WriteByte((byte)large);Span<byte>b=stackalloc byte[2];BinaryPrimitives.WriteUInt16BigEndian(b,checked((ushort)n));s.Write(b);
    }
    static void Write(Stream s,object? value) {
        switch(value) {
            case null:s.WriteByte(0xc0);break;
            case bool b:s.WriteByte(b?(byte)0xc3:(byte)0xc2);break;
            case byte[] bytes:Length(s,bytes.Length,0,0xc4,0xc5);s.Write(bytes);break;
            case string text:var utf=Encoding.UTF8.GetBytes(text);if(utf.Length<32)s.WriteByte((byte)(0xa0+utf.Length));else Length(s,utf.Length,0,0xd9,0xda);s.Write(utf);break;
            case IReadOnlyDictionary<string,object> map:Length(s,map.Count,0x80,0,0xde);foreach(var pair in map){Write(s,pair.Key);Write(s,pair.Value);}break;
            case System.Collections.IEnumerable items:var values=items.Cast<object?>().ToArray();Length(s,values.Length,0x90,0,0xdc);foreach(var item in values)Write(s,item);break;
            default:Number(s,Convert.ToInt64(value));break;
        }
    }
    public static JsonNode? Decode(byte[] body) {try{int position=0;var node=Read(body,ref position,0);if(position!=body.Length)throw new InvalidDataException("Trailing MessagePack data.");return node;}catch(OverflowException e){throw new InvalidDataException("Oversized MessagePack frame.",e);}}
    static JsonNode? Read(byte[] b,ref int p,int depth) {
        if(depth>40||p>=b.Length)throw new InvalidDataException("Invalid MessagePack frame.");
        byte code=b[p++];
        if(code<128)return JsonValue.Create((long)code);
        if(code>=224)return JsonValue.Create((long)unchecked((sbyte)code));
        if((code&0xe0)==0xa0)return JsonValue.Create(Text(b,ref p,code&31));
        if((code&0xf0)==0x90)return Array(b,ref p,code&15,depth);
        if((code&0xf0)==0x80)return Map(b,ref p,code&15,depth);
        switch(code) {
            case 0xc0:return null;case 0xc2:return JsonValue.Create(false);case 0xc3:return JsonValue.Create(true);
            case 0xcc:return JsonValue.Create((long)Unsigned(b,ref p,1));case 0xcd:return JsonValue.Create((long)Unsigned(b,ref p,2));case 0xce:return JsonValue.Create((long)Unsigned(b,ref p,4));case 0xcf:return JsonValue.Create(Unsigned(b,ref p,8));
            case 0xd0:return JsonValue.Create((long)unchecked((sbyte)Unsigned(b,ref p,1)));case 0xd1:return JsonValue.Create((long)unchecked((short)Unsigned(b,ref p,2)));case 0xd2:return JsonValue.Create((long)unchecked((int)Unsigned(b,ref p,4)));case 0xd3:return JsonValue.Create(unchecked((long)Unsigned(b,ref p,8)));
            case 0xd9:return JsonValue.Create(Text(b,ref p,(int)Unsigned(b,ref p,1)));case 0xda:return JsonValue.Create(Text(b,ref p,(int)Unsigned(b,ref p,2)));case 0xdb:return JsonValue.Create(Text(b,ref p,checked((int)Unsigned(b,ref p,4))));
            case 0xdc:return Array(b,ref p,(int)Unsigned(b,ref p,2),depth);case 0xdd:return Array(b,ref p,checked((int)Unsigned(b,ref p,4)),depth);
            case 0xde:return Map(b,ref p,(int)Unsigned(b,ref p,2),depth);case 0xdf:return Map(b,ref p,checked((int)Unsigned(b,ref p,4)),depth);
            case 0xc4:case 0xc5:case 0xc6:var len=checked((int)Unsigned(b,ref p,code==0xc4?1:code==0xc5?2:4));Check(b,p,len);var arr=new JsonArray();for(int n=0;n<len;n++)arr.Add((long)b[p++]);return arr;
            default:throw new InvalidDataException($"Unsupported MessagePack type {code:X2}.");
        }
    }
    static ulong Unsigned(byte[] b,ref int p,int count){Check(b,p,count);ulong v=0;for(int n=0;n<count;n++)v=(v<<8)|b[p++];return v;}
    static void Check(byte[]b,int p,int count){if(count<0||count>1_000_000||p>b.Length-count)throw new InvalidDataException("Truncated MessagePack frame.");}
    static string Text(byte[]b,ref int p,int count){Check(b,p,count);var s=Encoding.UTF8.GetString(b,p,count);p+=count;return s;}
    static JsonArray Array(byte[]b,ref int p,int count,int depth){if(count>4096)throw new InvalidDataException("Array too large.");var a=new JsonArray();for(int n=0;n<count;n++)a.Add(Read(b,ref p,depth+1));return a;}
    static JsonObject Map(byte[]b,ref int p,int count,int depth){if(count>4096)throw new InvalidDataException("Map too large.");var map=new JsonObject();for(int n=0;n<count;n++){var key=Read(b,ref p,depth+1)?.GetValue<string>()??throw new InvalidDataException("Map key is missing.");map[key]=Read(b,ref p,depth+1);}return map;}
    public static IEnumerable<Module> Modules(JsonNode? root) {
        if(root is not JsonObject obj)yield break;
        if(obj["u"] is JsonValue uid&&obj["i"] is JsonValue i&&obj["t"] is JsonValue t)yield return new(uid.GetValue<string>(),checked((int)i.GetValue<long>()),checked((int)t.GetValue<long>()));
        if(obj["c"] is JsonArray children)foreach(var child in children)foreach(var module in Modules(child))yield return module;
    }
    public static int Axis(int hi,int lo)=>unchecked((short)((hi<<8)|lo));
    public static long Color(string hex,int sub) {
        var value=Convert.ToUInt32(hex,16);return ((long)sub<<24)|((value>>16)&255)|(value&0xff00)|((value&255)<<16);
    }
}
public sealed class Framer {
    readonly List<byte> frame=[];bool inside,escaped;
    public IEnumerable<JsonNode> Feed(byte[] bytes) {
        var result=new List<JsonNode>();
        foreach(var b in bytes) {
            if(b==0x7e) {
                if(inside&&frame.Count>0){try{if(Wire.Decode(frame.ToArray()) is {} node)result.Add(node);}catch(InvalidDataException){}catch(InvalidOperationException){}}
                inside=true;escaped=false;frame.Clear();continue;
            }
            if(!inside)continue;
            if(escaped){frame.Add((byte)(b^0x20));escaped=false;}else if(b==0x7d)escaped=true;else frame.Add(b);
            if(frame.Count>1_000_000){inside=false;escaped=false;frame.Clear();}
        }
        return result;
    }
}
