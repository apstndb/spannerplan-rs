using System.Reflection;
using System.Runtime.InteropServices;

namespace SpannerPlan;

internal static class SpannerPlanNative
{
    static SpannerPlanNative()
    {
        NativeLibrary.SetDllImportResolver(typeof(SpannerPlanNative).Assembly, ResolveLibrary);
        SpannerPlanLibrary.EnsureResolver();
    }

    private static IntPtr ResolveLibrary(string libraryName, Assembly assembly, DllImportSearchPath? searchPath)
    {
        if (libraryName != SpannerPlanLibrary.LibName)
        {
            return IntPtr.Zero;
        }

        return NativeLibrary.Load(SpannerPlanLibrary.LibraryPath);
    }

    [DllImport(SpannerPlanLibrary.LibName, CallingConvention = CallingConvention.Cdecl)]
    internal static extern IntPtr spannerplan_render_tree_table_json(
        [MarshalAs(UnmanagedType.LPUTF8Str)] string planJson,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string mode,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string format,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string? configJson,
        out int outIsError);

    [DllImport(SpannerPlanLibrary.LibName, CallingConvention = CallingConvention.Cdecl)]
    internal static extern IntPtr spannerplan_render_tree_table_wire(
        byte[] planWire,
        nuint planWireLen,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string mode,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string format,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string? configJson,
        out int outIsError);

    [DllImport(SpannerPlanLibrary.LibName, CallingConvention = CallingConvention.Cdecl)]
    internal static extern void spannerplan_string_free(IntPtr s);

    internal delegate IntPtr RenderDelegate(out int outIsError);

    internal static string CallRender(RenderDelegate render)
    {
        var ptr = render(out var isError);
        if (ptr == IntPtr.Zero)
        {
            throw new RenderError("native render returned NULL");
        }

        try
        {
            var text = Marshal.PtrToStringUTF8(ptr);
            if (text is null)
            {
                throw new RenderError("native render returned invalid UTF-8");
            }

            if (isError != 0)
            {
                throw new RenderError(text);
            }

            return text;
        }
        finally
        {
            spannerplan_string_free(ptr);
        }
    }

    internal static string? ToConfigJson(RenderConfig? config)
    {
        if (config is null || config.Count == 0)
        {
            return null;
        }

        return System.Text.Json.JsonSerializer.Serialize(config);
    }
}
