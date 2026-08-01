namespace Inquivora.Native.Audio;

public static class AudioMath
{
    public const double SilenceThreshold = 0.005;

    public static double Rms(float[] samples)
    {
        if (samples.Length == 0)
        {
            return 0.0;
        }
        double sum = 0;
        foreach (var sample in samples)
        {
            sum += (double)sample * sample;
        }
        return Math.Sqrt(sum / samples.Length);
    }

    public static bool IsSilent(float[] samples, double threshold = SilenceThreshold) =>
        Rms(samples) < threshold;

    public static float[] ApplyGain(float[] samples, double gain)
    {
        var adjusted = new float[samples.Length];
        var safeGain = double.IsFinite(gain) ? Math.Clamp(gain, 0.5, 4.0) : 1.0;
        for (var i = 0; i < samples.Length; i++)
        {
            adjusted[i] = Math.Clamp((float)(samples[i] * safeGain), -1.0f, 1.0f);
        }
        return adjusted;
    }
}
