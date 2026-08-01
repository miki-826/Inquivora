using Inquivora.Native.Audio;
using Xunit;

namespace Inquivora.Native.Tests;

public class AudioConversionTests
{
    [Fact]
    public void ダウンサンプリングで長さが比率どおりになる()
    {
        var input = new float[48000];
        var output = LinearResampler.Resample(input, 48000, 16000);
        Assert.Equal(16000, output.Length);
    }

    [Fact]
    public void 一定値の信号は保持される()
    {
        var input = new float[4800];
        Array.Fill(input, 0.25f);
        var output = LinearResampler.Resample(input, 48000, 16000);
        Assert.All(output, sample => Assert.Equal(0.25f, sample, 2));
    }

    [Fact]
    public void 同一レートなら入力がそのまま返る()
    {
        var input = new float[] { 0.1f, 0.2f, 0.3f };
        var output = LinearResampler.Resample(input, 16000, 16000);
        Assert.Equal(input, output);
    }

    [Fact]
    public void ステレオをモノラルへ平均できる()
    {
        var interleaved = new float[] { 1.0f, 0.0f, 0.5f, 0.5f, -1.0f, 1.0f };
        var mono = LinearResampler.ToMono(interleaved, channels: 2);
        Assert.Equal(new[] { 0.5f, 0.5f, 0.0f }, mono);
    }

    [Fact]
    public void StrongestChannelMonoPreservesSingleChannelMicrophoneLevel()
    {
        var interleaved = new float[] { 0.4f, 0.0f, -0.2f, 0.0f };
        var mono = LinearResampler.ToStrongestChannelMono(interleaved, channels: 2);
        Assert.Equal(new[] { 0.4f, -0.2f }, mono);
    }

    [Fact]
    public void GainAmplifiesAndClampsSamples()
    {
        var adjusted = AudioMath.ApplyGain(new float[] { 0.1f, -0.4f }, 3.0);
        Assert.Equal(0.3f, adjusted[0], 3);
        Assert.Equal(-1.0f, adjusted[1], 3);
    }

    [Fact]
    public void Wavヘッダーは16bitモノラル16kHzを示す()
    {
        var dir = Directory.CreateTempSubdirectory();
        var path = Path.Combine(dir.FullName, "test.wav");
        try
        {
            var samples = new float[16000];
            Array.Fill(samples, 0.5f);
            WavFile.WritePcm16Mono(path, samples, 16000);
            var bytes = File.ReadAllBytes(path);
            Assert.Equal("RIFF", System.Text.Encoding.ASCII.GetString(bytes, 0, 4));
            Assert.Equal("WAVE", System.Text.Encoding.ASCII.GetString(bytes, 8, 4));
            Assert.Equal(1, BitConverter.ToInt16(bytes, 22));
            Assert.Equal(16000, BitConverter.ToInt32(bytes, 24));
            Assert.Equal(16, BitConverter.ToInt16(bytes, 34));
            Assert.Equal(44 + 16000 * 2, bytes.Length);
        }
        finally
        {
            dir.Delete(recursive: true);
        }
    }

    [Fact]
    public void Wavのサンプル値はクリップされて書き込まれる()
    {
        var dir = Directory.CreateTempSubdirectory();
        var path = Path.Combine(dir.FullName, "clip.wav");
        try
        {
            WavFile.WritePcm16Mono(path, new float[] { 2.0f, -2.0f }, 16000);
            var bytes = File.ReadAllBytes(path);
            Assert.Equal(short.MaxValue, BitConverter.ToInt16(bytes, 44));
            Assert.Equal(short.MinValue, BitConverter.ToInt16(bytes, 46));
        }
        finally
        {
            dir.Delete(recursive: true);
        }
    }
}
