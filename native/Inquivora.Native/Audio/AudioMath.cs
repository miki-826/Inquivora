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
}
