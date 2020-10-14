fn main() {
    tonic_build::compile_protos("proto/control.proto").unwrap();
    tonic_build::compile_protos("proto/sentry.proto").unwrap();
}
