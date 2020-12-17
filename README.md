# cnat

Cloud Native [`at`], runs `command` at `schedule` by creating a Pod.

```yaml
apiVersion: example.kazk.dev/v1alpha1
kind: At
metadata:
  name: example
spec:
  schedule: "2020-12-17T19:21:00Z"
  command: [echo, "will be executed"]
```

Example Kubernetes controller [programming-kubernetes/cnat] in Rust using [kube].

## Changes

- Changed `command` to an array
- Removed `Pending` from `status.phase`. `None` -> `Running` -> `Done`
- Changed to use `apiextensions.k8s.io/v1` and added the required `schemas`
  - Note that `kube` currently does not support schema generation ([clux/kube-rs#264])
- Changed to automatically install the current CRD if not already installed

## TODO

- [ ] Write tests
- [ ] Set up GitHub Actions with k3d

## Comparisons

### Binary Size

Building on Arch Linux:

- `client-go`: 31MB (23MB with `-ldflags "-s -w"`)
- `kubebuilder`: TBD requires unknown version of [KubeBuilder]
- `operator-sdk`: TBD requires unknown version of [Operator SDK]
- `kube`: 11MB (8.2MB with `RUSTFLAGS='-C link-arg=-s'`)

### Lines of Code

|   Library     |  Files   |  Lines  |   Code  |  Comments |  Blanks |
| :-----------: | -------: | ------: | ------: | --------: | ------: | 
| client-go     |    29    |   2161  |   1212  |       658 |     291 |
| kubebuilder   |    16    |   1013  |    529  |       357 |     127 |
| operator-sdk  |    13    |    714  |    488  |       134 |      92 |
| kube          |     4    |    394  |    327  |        29 |      38 |


- `client-go`: `tokei -e cnat-client-go/vendor -e cnat-client-go/hack -t Go cnat-client-go`
- `kubebuilder`: `tokei -e cnat-kubebuilder/vendor -e cnat-kubebuilder/hack -t Go cnat-kubebuilder`
- `operator-sdk`: `tokei -e cnat-operator/vendor -t Go cnat-operator`
- `kube`: `tokei -t Rust`

### Project Setup and Dependencies

- `client-go`: Go modules works fine (3 core modules + 1 code geerator + 1 logger). Versioning has improved since [programming-kubernetes/cnat] was written. A lot of generated code.
- `kubebuilder`: Requires [KubeBuilder] tool. Build failed with `GOPATH` not defined. Uses [dep].
- `operator-sdk`: Requires [Operator SDK] tool. Haven't tried. Uses [dep].
- `kube`: Standard Rust project. No separate codegen and very straightforward. Currently a workaround is necessary to create CRD with `apiextensions.k8s.io/v1` ([clux/kube-rs#264]). Maybe [schemers](https://github.com/GREsau/schemars) can be used?

## References

- [ConfigMapGenerator example](https://github.com/clux/kube-rs/blob/f49fcc4b64ca53091efe15f570e38c6ab3789567/examples/configmapgen_controller.rs)
- [at_controller.go](https://github.com/programming-kubernetes/cnat/blob/27f8ddba657b803ffb10501a28e003d0febd6387/cnat-kubebuilder/pkg/controller/at/at_controller.go)
- [h2oai/h2o-kubernetes](https://github.com/h2oai/h2o-kubernetes)


[`at`]: https://en.wikipedia.org/wiki/At_(command)
[programming-kubernetes/cnat]: https://github.com/programming-kubernetes/cnat
[KubeBuilder]: https://book.kubebuilder.io/quick-start.html
[Operator SDK]: https://github.com/operator-framework/operator-sdk
[kube]: https://github.com/clux/kube-rs
[clux/kube-rs#264]: https://github.com/clux/kube-rs/issues/264
[dep]: https://github.com/golang/dep
