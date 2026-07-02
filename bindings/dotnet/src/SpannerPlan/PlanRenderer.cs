namespace SpannerPlan;

/// <summary>
/// High-level API for rendering Cloud Spanner query plans via the spannerplan-ffi C ABI.
/// </summary>
public static class PlanRenderer
{
    /// <summary>
    /// Render from JSON/YAML text (QueryPlan, ResultSetStats, or ResultSet shapes).
    /// </summary>
    public static string RenderTreeTableJson(
        string planJson,
        string mode = "AUTO",
        string format = "CURRENT",
        RenderConfig? config = null)
    {
        var configJson = SpannerPlanNative.ToConfigJson(config);

        return SpannerPlanNative.CallRender((out int isError) =>
            SpannerPlanNative.spannerplan_render_tree_table_json(
                planJson,
                mode,
                format,
                configJson,
                out isError));
    }

    /// <summary>
    /// Render from protobuf wire-encoded plan bytes.
    /// </summary>
    public static string RenderTreeTableWire(
        ReadOnlySpan<byte> planWire,
        string mode = "AUTO",
        string format = "CURRENT",
        RenderConfig? config = null)
    {
        var planBytes = planWire.ToArray();
        var configJson = SpannerPlanNative.ToConfigJson(config);

        return SpannerPlanNative.CallRender((out int isError) =>
            SpannerPlanNative.spannerplan_render_tree_table_wire(
                planBytes,
                (nuint)planBytes.Length,
                mode,
                format,
                configJson,
                out isError));
    }
}
