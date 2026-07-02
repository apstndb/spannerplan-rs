package io.spannerplan;

import com.sun.jna.Pointer;
import com.sun.jna.ptr.IntByReference;
import java.util.Map;
import java.util.Objects;
import java.util.function.Function;

/** High-level API for rendering Cloud Spanner query plans. */
public final class Spannerplan {
  private Spannerplan() {}

  /**
   * Render from JSON/YAML text (QueryPlan, ResultSetStats, or ResultSet shapes).
   *
   * @param planJson plan text
   * @param mode render mode (e.g. {@code AUTO})
   * @param format render format (e.g. {@code CURRENT})
   * @param config optional render options serialized to JSON for the FFI layer
   * @return rendered ASCII table
   */
  public static String renderTreeTableJson(
      String planJson, String mode, String format, Map<String, ?> config) {
    Objects.requireNonNull(planJson, "planJson");
    Objects.requireNonNull(mode, "mode");
    Objects.requireNonNull(format, "format");
    return callRender(
        isError ->
            SpannerplanNative.INSTANCE.spannerplan_render_tree_table_json(
                planJson, mode, format, configJson(config), isError));
  }

  /** Render from JSON/YAML with default mode {@code AUTO} and format {@code CURRENT}. */
  public static String renderTreeTableJson(String planJson) {
    return renderTreeTableJson(planJson, "AUTO", "CURRENT", null);
  }

  /**
   * Render from protobuf wire-encoded plan bytes.
   *
   * @param planWire wire bytes
   * @param mode render mode (e.g. {@code AUTO})
   * @param format render format (e.g. {@code CURRENT})
   * @param config optional render options serialized to JSON for the FFI layer
   * @return rendered ASCII table
   */
  public static String renderTreeTableWire(
      byte[] planWire, String mode, String format, Map<String, ?> config) {
    Objects.requireNonNull(planWire, "planWire");
    Objects.requireNonNull(mode, "mode");
    Objects.requireNonNull(format, "format");
    return callRender(
        isError ->
            SpannerplanNative.INSTANCE.spannerplan_render_tree_table_wire(
                planWire, planWire.length, mode, format, configJson(config), isError));
  }

  /** Render from protobuf wire bytes with default mode {@code AUTO} and format {@code CURRENT}. */
  public static String renderTreeTableWire(byte[] planWire) {
    return renderTreeTableWire(planWire, "AUTO", "CURRENT", null);
  }

  private static String callRender(Function<IntByReference, Pointer> renderFn) {
    IntByReference isError = new IntByReference();
    Pointer out = renderFn.apply(isError);
    if (out == null) {
      throw new RenderError("native render returned NULL");
    }
    try {
      String text = out.getString(0);
      if (isError.getValue() != 0) {
        throw new RenderError(text);
      }
      return text;
    } finally {
      SpannerplanNative.INSTANCE.spannerplan_string_free(out);
    }
  }

  private static String configJson(Map<String, ?> config) {
    if (config == null || config.isEmpty()) {
      return null;
    }
    StringBuilder sb = new StringBuilder("{");
    boolean first = true;
    for (Map.Entry<String, ?> entry : config.entrySet()) {
      if (!first) {
        sb.append(',');
      }
      first = false;
      sb.append('"').append(escapeJson(entry.getKey())).append("\":");
      sb.append(jsonValue(entry.getValue()));
    }
    sb.append('}');
    return sb.toString();
  }

  private static String jsonValue(Object value) {
    if (value == null) {
      return "null";
    }
    if (value instanceof Boolean b) {
      return b ? "true" : "false";
    }
    if (value instanceof Number n) {
      return n.toString();
    }
    return "\"" + escapeJson(String.valueOf(value)) + "\"";
  }

  private static String escapeJson(String text) {
    return text.replace("\\", "\\\\").replace("\"", "\\\"");
  }
}
