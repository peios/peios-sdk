"""Finish C ABI archives without exposing their private Rust runtime symbols."""

def _isolated_static_impl(ctx):
    output = ctx.actions.declare_file(ctx.label.name + "/lib" + ctx.attr.prefix + ".a")
    ctx.actions.run_shell(
        inputs = [ctx.file.source, ctx.file._tool],
        outputs = [output],
        command = 'exec python3 "$1" "$2" "$3" --prefix "$4"',
        arguments = [ctx.file._tool.path, ctx.file.source.path, output.path, ctx.attr.prefix],
        mnemonic = "IsolateStaticRuntime",
        progress_message = "Isolating private runtime symbols in %{label}",
        use_default_shell_env = True,
    )
    return [DefaultInfo(files = depset([output]))]

isolated_static = rule(
    implementation = _isolated_static_impl,
    attrs = {
        "source": attr.label(allow_single_file = [".a"], mandatory = True),
        "prefix": attr.string(mandatory = True, values = ["peios", "rsi"]),
        "_tool": attr.label(default = Label("//:tools/isolate-static.py"), allow_single_file = True),
    },
    doc = "Namespace private definitions using the declared host binutils build dependency.",
)
