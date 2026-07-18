namespace Inquivora.Native.Audio;

/// <summary>
/// §9.6 API送信用形式（PCM WAV・mono・16bit）の書き出し。
/// </summary>
public static class WavFile
{
    public static void WritePcm16Mono(string path, float[] samples, int sampleRate)
    {
        using var stream = File.Create(path);
        using var writer = new BinaryWriter(stream);
        var dataLength = samples.Length * 2;
        writer.Write("RIFF"u8);
        writer.Write(36 + dataLength);
        writer.Write("WAVE"u8);
        writer.Write("fmt "u8);
        writer.Write(16);
        writer.Write((short)1);
        writer.Write((short)1);
        writer.Write(sampleRate);
        writer.Write(sampleRate * 2);
        writer.Write((short)2);
        writer.Write((short)16);
        writer.Write("data"u8);
        writer.Write(dataLength);
        foreach (var sample in samples)
        {
            var scaled = (int)Math.Round(sample * 32768.0);
            writer.Write((short)Math.Clamp(scaled, short.MinValue, short.MaxValue));
        }
    }
}
