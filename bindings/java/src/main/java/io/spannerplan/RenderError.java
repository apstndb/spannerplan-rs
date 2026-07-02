package io.spannerplan;

/** Raised when the native renderer returns an error string. */
public class RenderError extends RuntimeException {
  public RenderError(String message) {
    super(message);
  }
}
