package io.spannerplan;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import org.junit.jupiter.api.Test;

class RenderTest {
  private static final Path REPO_ROOT =
      Paths.get(System.getProperty("user.dir")).getParent().getParent();
  private static final Path FIXTURE = REPO_ROOT.resolve("testdata/reference/dca.yaml");
  private static final Path GOLDEN = REPO_ROOT.resolve("testdata/golden/dca_plan_current.txt");

  @Test
  void renderFixtureContainsDistributedCrossApply() throws Exception {
    String plan = Files.readString(FIXTURE, StandardCharsets.UTF_8);
    String output = Spannerplan.renderTreeTableJson(plan);

    assertTrue(output.contains("Distributed Cross Apply"));
  }

  @Test
  void renderGoldenMatchesDcaPlanCurrent() throws Exception {
    String plan = Files.readString(FIXTURE, StandardCharsets.UTF_8);
    String output = Spannerplan.renderTreeTableJson(plan, "PLAN", "CURRENT", null);
    String golden = Files.readString(GOLDEN, StandardCharsets.UTF_8);

    assertEquals(golden, output);
  }

  @Test
  void renderInvalidJsonRaises() {
    RenderError error =
        assertThrows(RenderError.class, () -> Spannerplan.renderTreeTableJson("not json"));
    assertTrue(!error.getMessage().isBlank());
  }
}
