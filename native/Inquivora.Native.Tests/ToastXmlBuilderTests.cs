using Inquivora.Native.Notifications;
using Xunit;

namespace Inquivora.Native.Tests;

public class ToastXmlBuilderTests
{
    [Fact]
    public void protocol起動と本文を含むトーストXMLを生成する()
    {
        var xml = ToastXmlBuilder.Build("予定開始", "定例会が10:00から始まります。", "inquivora://open?type=event&id=e1", false);
        Assert.Contains("activationType=\"protocol\"", xml);
        Assert.Contains("launch=\"inquivora://open?type=event&amp;id=e1\"", xml);
        Assert.Contains("<text>予定開始</text>", xml);
        Assert.Contains("<text>定例会が10:00から始まります。</text>", xml);
        Assert.DoesNotContain("<audio", xml);
    }

    [Fact]
    public void xml特殊文字をエスケープする()
    {
        var xml = ToastXmlBuilder.Build("<注意> & \"引用\"", "a < b & c", "inquivora://open?type=task&id=t1", false);
        Assert.Contains("&lt;注意&gt; &amp; &quot;引用&quot;", xml);
        Assert.Contains("<text>a &lt; b &amp; c</text>", xml);
        Assert.DoesNotContain("<注意>", xml);
    }

    [Fact]
    public void silent指定でaudio要素が入る()
    {
        var xml = ToastXmlBuilder.Build("t", "b", "inquivora://open?type=task&id=t1", true);
        Assert.Contains("<audio silent=\"true\"/>", xml);
    }

    [Fact]
    public void inquivora以外のスキームは拒否する()
    {
        Assert.Throws<ArgumentException>(
            () => ToastXmlBuilder.Build("t", "b", "https://example.com/", false));
    }

    [Fact]
    public void uriとして不正な値は拒否する()
    {
        Assert.Throws<ArgumentException>(() => ToastXmlBuilder.Build("t", "b", "ただの文字列", false));
    }
}
