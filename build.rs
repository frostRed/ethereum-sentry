fn main() {
    tonic_build::compile_protos("proto/p2psentry/control.proto").unwrap();
    tonic_build::compile_protos("proto/p2psentry/sentry.proto").unwrap();
}
