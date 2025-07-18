#!/usr/bin/env -S nu --stdin

def --wrapped main [rustc: string, ...args] {
    if ($args | any {str starts-with "--print"}) or ($args == ["-vV"]) {
        run-external $rustc ...$args
        return;
    }
    
    if ("CARGO_PKG_NAME" in $env and $env.CARGO_PKG_NAME == "aubystd") {
        $env.RUSTC_BOOTSTRAP = 1
        run-external $rustc ...$args
        return;
    }
    
    let metadata = run-external $env.CARGO metadata "--manifest-path" $env.CARGO_MANIFEST_PATH "--format-version" 1 | complete | get stdout | from json
    
    let found_aubystd_in_deps = not ($metadata.resolve.nodes
        | find -n ($metadata.resolve.root)
        | first
        | get deps
        | find -nc ["name"] -r "^aubystd$"
        | is-empty);

    if (not $found_aubystd_in_deps) {
        run-external $rustc ...$args
        return;
    }
    
    let targets = $metadata.packages | find -nc [id] $metadata.resolve.root | first | get targets;
    
    let target = if ("CARGO_BIN_NAME" in $env) {
        $targets | find -nc [name] $env.CARGO_BIN_NAME | first
    } else {
        $targets | where {|t| ($t.name | str replace -a '-' '_') == $env.CARGO_CRATE_NAME} | first
    };

    let target_filename = $target.src_path;
    let args_filename = $args | where {|a| ($a | path exists) and ($target_filename | str ends-with $a)} | first
    let new_input = mktemp -t;
    let new_args = $args | each {|a| if ($a == $args_filename) { $new_input } else { $a } };
    if (not ($new_args | any {str contains "--json"})) {
        
    }

    try {
        # $new_input | print
        const prefix_path = path self ./prefix.rs
        let original = open -r $target_filename | lines;
        # $original | print
        let inner_attrs = $original | take while {str trim | str starts-with '#!['}
        # if (not ($inner_attrs | any {str contains {}} |)) {

        # }
        $inner_attrs | str join (char nl) | save -fr $new_input
        # open $new_input -r | lines | print
        let prefix = $prefix_path | open -r | str replace -ar "\n" "" | save -ar $new_input
        # open $new_input -r | lines | print
        $original | skip ($inner_attrs | length) | save -ar $new_input 
        # open $new_input -r | lines | print
        
        $env.RUSTC_BOOTSTRAP = 1
        let output = if (run-external $rustc "-vV" | str contains nightly) {
            run-external $rustc ...$new_args | complete
        } else { 
            "-Zallow-features=prelude_import,no_core,more_qualified_paths"
            run-external $rustc ...$new_args | complete
        };
        $output | get stderr | from json -o | each {|x|
            if (($x | get rendered -i) != null) {
                mut y = $x;
                $y.rendered = $x.rendered | str replace -a $new_input $args_filename
                $y
            } else {
                $x
            }
        } | each {to json -r | print -e};
        rm -f $new_input
        exit $output.exit_code
    } catch {|e|
        rm -f $new_input
        $e.rendered | print
        exit 1
    }
}
