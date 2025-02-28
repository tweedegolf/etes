fn main() {
    spaxum::bundle_with_args(
        "./frontend/src/main.tsx",
        &[
            "--jsx-factory=preact.h",
            "--jsx-fragment=preact.Fragment",
            "--alias:react=preact/compat",
        ],
    );
}
