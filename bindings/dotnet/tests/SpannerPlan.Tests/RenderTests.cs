using SpannerPlan;
using Xunit;

namespace SpannerPlan.Tests;

public sealed class RenderTests
{
    [Fact]
    public void RenderFixture_ContainsDistributedCrossApply()
    {
        var yaml = File.ReadAllText(TestPaths.Fixture("reference/dca.yaml"));
        var output = PlanRenderer.RenderTreeTableJson(yaml);

        Assert.Contains("Distributed Cross Apply", output);
    }

    [Fact]
    public void RenderGolden_MatchesDcaPlanCurrent()
    {
        var yaml = File.ReadAllText(TestPaths.Fixture("reference/dca.yaml"));
        var output = PlanRenderer.RenderTreeTableJson(yaml, mode: "PLAN", format: "CURRENT");
        var expected = File.ReadAllText(TestPaths.Golden("dca_plan_current"));

        Assert.Equal(expected, output);
    }

    [Fact]
    public void RenderInvalidJson_ThrowsRenderError()
    {
        var ex = Assert.Throws<RenderError>(() =>
            PlanRenderer.RenderTreeTableJson("not json"));

        Assert.False(string.IsNullOrWhiteSpace(ex.Message));
    }
}
