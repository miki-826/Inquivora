using Inquivora.Native.Audio;
using Xunit;

namespace Inquivora.Native.Tests;

public class AudioChunkerTests
{
    private const int SampleRate = 16000;

    private static float[] Samples(int seconds, float value = 0.1f)
    {
        var samples = new float[SampleRate * seconds];
        Array.Fill(samples, value);
        return samples;
    }

    [Fact]
    public void 二十秒たまるとチャンクが出る()
    {
        var chunker = new AudioChunker(SampleRate, chunkSeconds: 20, overlapSeconds: 1);
        Assert.Empty(chunker.Add(Samples(19)));
        var chunks = chunker.Add(Samples(1));
        var chunk = Assert.Single(chunks);
        Assert.Equal(SampleRate * 20, chunk.Samples.Length);
        Assert.Equal(0, chunk.StartMs);
        Assert.Equal(20000, chunk.EndMs);
    }

    [Fact]
    public void 次のチャンクは1秒オーバーラップする()
    {
        var chunker = new AudioChunker(SampleRate, chunkSeconds: 20, overlapSeconds: 1);
        chunker.Add(Samples(20));
        var chunks = chunker.Add(Samples(19));
        var chunk = Assert.Single(chunks);
        Assert.Equal(19000, chunk.StartMs);
        Assert.Equal(39000, chunk.EndMs);
        Assert.Equal(SampleRate * 20, chunk.Samples.Length);
    }

    [Fact]
    public void 長い入力から複数チャンクが出る()
    {
        var chunker = new AudioChunker(SampleRate, chunkSeconds: 20, overlapSeconds: 1);
        var chunks = chunker.Add(Samples(60));
        Assert.Equal(3, chunks.Count);
        Assert.Equal(0, chunks[0].StartMs);
        Assert.Equal(19000, chunks[1].StartMs);
        Assert.Equal(38000, chunks[2].StartMs);
    }

    [Fact]
    public void Flushで端数チャンクが出る()
    {
        var chunker = new AudioChunker(SampleRate, chunkSeconds: 20, overlapSeconds: 1);
        chunker.Add(Samples(25));
        var rest = chunker.Flush();
        Assert.NotNull(rest);
        Assert.Equal(19000, rest!.StartMs);
        Assert.Equal(25000, rest.EndMs);
    }

    [Fact]
    public void 一秒未満の端数はflushで捨てられる()
    {
        var chunker = new AudioChunker(SampleRate, chunkSeconds: 20, overlapSeconds: 1);
        chunker.Add(Samples(20));
        var samples = new float[SampleRate / 2];
        chunker.Add(samples);
        Assert.Null(chunker.Flush());
    }

    [Fact]
    public void 無音を判定できる()
    {
        Assert.True(AudioMath.IsSilent(new float[SampleRate]));
        Assert.False(AudioMath.IsSilent(Samples(1, 0.1f)));
    }

    [Fact]
    public void Rmsを計算できる()
    {
        var samples = new float[] { 0.5f, -0.5f, 0.5f, -0.5f };
        Assert.Equal(0.5, AudioMath.Rms(samples), 3);
        Assert.Equal(0.0, AudioMath.Rms(Array.Empty<float>()), 3);
    }
}
