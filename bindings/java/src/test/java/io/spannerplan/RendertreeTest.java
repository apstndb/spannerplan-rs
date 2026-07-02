package io.spannerplan;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.io.ByteArrayInputStream;
import java.io.ByteArrayOutputStream;
import java.io.PrintStream;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import org.junit.jupiter.api.Test;

class RendertreeTest {
  private static final Path REPO_ROOT =
      Paths.get(System.getProperty("user.dir")).getParent().getParent();
  private static final Path FIXTURE = REPO_ROOT.resolve("testdata/reference/dca.yaml");

  @Test
  void helpExitsZero() throws Exception {
    Capture capture = run(new String[] {"-h"}, new byte[0]);
    assertEquals(0, capture.exitCode);
    assertTrue(capture.stderr.contains("-mode"));
    assertTrue(capture.stdout.isEmpty());
  }

  @Test
  void unknownFlagExitsTwo() throws Exception {
    Capture capture = run(new String[] {"-unknown"}, new byte[0]);
    assertEquals(2, capture.exitCode);
    assertTrue(capture.stderr.contains("flag provided but not defined"));
    assertTrue(capture.stderr.contains("Usage of rendertree:"));
  }

  @Test
  void renderFixture() throws Exception {
    byte[] input = Files.readAllBytes(FIXTURE);
    Capture capture = run(new String[] {"-mode", "plan"}, input);
    assertEquals(0, capture.exitCode);
    assertTrue(capture.stdout.contains("Distributed Cross Apply"));
    assertTrue(capture.stderr.isEmpty());
  }

  @Test
  void invalidModeExitsTwo() throws Exception {
    byte[] input = Files.readAllBytes(FIXTURE);
    Capture capture = run(new String[] {"-mode", "bogus"}, input);
    assertEquals(2, capture.exitCode);
    assertTrue(capture.stderr.contains("Invalid value for -mode flag"));
  }

  private static Capture run(String[] args, byte[] stdin) throws Exception {
    ByteArrayOutputStream stdout = new ByteArrayOutputStream();
    ByteArrayOutputStream stderr = new ByteArrayOutputStream();
    int exitCode =
        Rendertree.run(
            args,
            new ByteArrayInputStream(stdin),
            new PrintStream(stdout, true, StandardCharsets.UTF_8),
            new PrintStream(stderr, true, StandardCharsets.UTF_8));
    return new Capture(
        exitCode,
        stdout.toString(StandardCharsets.UTF_8),
        stderr.toString(StandardCharsets.UTF_8));
  }

  private record Capture(int exitCode, String stdout, String stderr) {}
}
