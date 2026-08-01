namespace Inquivora.Native.Audio;

/// <summary>
/// 線形補間による簡易リサンプラー。音声認識用途（16kHz mono）には十分な品質。
/// </summary>
public static class LinearResampler
{
    public static float[] Resample(float[] input, int fromRate, int toRate)
    {
        if (fromRate == toRate || input.Length == 0)
        {
            return input;
        }
        var outputLength = (int)((long)input.Length * toRate / fromRate);
        var output = new float[outputLength];
        var step = (double)fromRate / toRate;
        for (var i = 0; i < outputLength; i++)
        {
            var position = i * step;
            var index = (int)position;
            var fraction = (float)(position - index);
            var next = Math.Min(index + 1, input.Length - 1);
            output[i] = input[index] + (input[next] - input[index]) * fraction;
        }
        return output;
    }

    public static float[] ToMono(float[] interleaved, int channels)
    {
        if (channels <= 1)
        {
            return interleaved;
        }
        var frames = interleaved.Length / channels;
        var mono = new float[frames];
        for (var frame = 0; frame < frames; frame++)
        {
            float sum = 0;
            for (var channel = 0; channel < channels; channel++)
            {
                sum += interleaved[frame * channels + channel];
            }
            mono[frame] = sum / channels;
        }
        return mono;
    }

    public static float[] ToStrongestChannelMono(float[] interleaved, int channels)
    {
        if (channels <= 1)
        {
            return interleaved;
        }
        var frames = interleaved.Length / channels;
        var sums = new double[channels];
        for (var frame = 0; frame < frames; frame++)
        {
            for (var channel = 0; channel < channels; channel++)
            {
                var sample = interleaved[frame * channels + channel];
                sums[channel] += (double)sample * sample;
            }
        }
        var strongest = 0;
        for (var channel = 1; channel < channels; channel++)
        {
            if (sums[channel] > sums[strongest]) strongest = channel;
        }
        var mono = new float[frames];
        for (var frame = 0; frame < frames; frame++)
        {
            mono[frame] = interleaved[frame * channels + strongest];
        }
        return mono;
    }
}
