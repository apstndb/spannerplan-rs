namespace SpannerPlan;

/// <summary>
/// Raised when the native renderer returns an error string.
/// </summary>
public sealed class RenderError : Exception
{
    internal RenderError(string message) : base(message)
    {
    }
}
