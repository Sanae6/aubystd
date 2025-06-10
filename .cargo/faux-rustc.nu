#!/usr/bin/env -S nu --stdin

def --wrapped main [rustc: string, ...args] {
    if ($args | any {str starts-with "--print"}) or ($args == ["-vV"]) {
        run-external $rustc ...$args
        return;
    }
    
    if ("CARGO_PKG_NAME" in $env and $env.CARGO_PKG_NAME == "aubystd") {
        # "wowaah" | print
        # exit 1
        $env.RUSTC_BOOTSTRAP = 1
        run-external $rustc ...$args
        return;
    }
    # ($rustc) | print
    # ($args) | print
    # ($env.CARGO_MANIFEST_PATH) | print
    
    let metadata = run-external $env.CARGO metadata "--manifest-path" $env.CARGO_MANIFEST_PATH "--format-version" 1 | complete | get stdout | from json
    
    let found_aubystd_in_deps = not ($metadata.resolve.nodes
        | find -n ($metadata.resolve.root)
        | first
        | get deps
        | find -nc ["name"] -r "^aubystd$"
        | is-empty);

    # $metadata.resolve.nodes
    #         | find -n ($metadata.resolve.root)
    #         | first
    #         | get deps | print
    # $found_aubystd_in_deps | print
    if (not $found_aubystd_in_deps) {
        run-external $rustc ...$args
        return;
    }
    
    let targets = $metadata.packages | find -nc [id] $metadata.resolve.root | first | get targets;
    
    # $targets | print

    let target = if ("CARGO_BIN_NAME" in $env) {
        $targets | find -nc [name] $env.CARGO_BIN_NAME | first
    } else {
        $targets | where {|t| ($t.name | str replace '-' '_') == $env.CARGO_CRATE_NAME} | first
    };
    # $target | print
    let target_filename = $target.src_path;
    let args_filename = $args | where {|a| ($a | path exists) and ($target_filename | str ends-with $a)}
    let new_args = $args | each {|a| if ($a == $args_filename) { $target_filename } else { $a } };
    let new_input = mktemp -t;
    try {
        # $new_input | print
        const prefix_path = path self ./prefix.rs
        $prefix_path | open | save -a $new_input
        (open -r $target_filename) | save -a $new_input
        
        # exit 1
        $env.RUSTC_BOOTSTRAP = 1
        if (rustc -vV | str contains nightly) {
            run-external $rustc ...$new_args
        } else { 
            "-Zallow-features=prelude_import,no_core,more_qualified_paths"
            run-external $rustc ...$new_args
        }
        rm -f $new_input
    } catch {|e|
        rm -f $new_input
        error make $e.raw
    }
}
