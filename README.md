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
- Changed to use `apiextensions.k8s.io/v1` and added the required `schema`
  - The schema is generated by `kube` ([clux/kube-rs#348])
  - `schedule` is validated as `date-time`

<details>
<summary>Generated CRD</summary>

```yaml
apiVersion: apiextensions.k8s.io/v1
kind: CustomResourceDefinition
metadata:
  name: ats.example.kazk.dev
spec:
  group: example.kazk.dev
  names:
    kind: At
    plural: ats
    shortNames: []
    singular: at
  scope: Namespaced
  versions:
    - additionalPrinterColumns: []
      name: v1alpha1
      schema:
        openAPIV3Schema:
          description: "Auto-generated derived type for AtSpec via `CustomResource`"
          properties:
            spec:
              description: Spec for custom resource At.
              properties:
                command:
                  items:
                    type: string
                  type: array
                schedule:
                  format: date-time
                  type: string
              required:
                - command
                - schedule
              type: object
            status:
              description: Status for custom resource At.
              nullable: true
              properties:
                phase:
                  description: Describes the status of the scheduled command.
                  enum:
                    - Running
                    - Done
                  type: string
              required:
                - phase
              type: object
          required:
            - spec
          title: At
          type: object
      served: true
      storage: true
      subresources:
        status: {}
```

</details>

## Project Structure

```text
.
├── k8s/               CRD and example
└── src/
    ├── bin/           Binaries
    │   ├── cnat.rs    - Run controller (default-run)
    │   └── crd.rs     - Output CRD YAML
    ├── controller.rs  Implements the Controller
    ├── lib.rs
    └── resource.rs    Defines the Custom Resource
```

### Commands

- `cargo run`: Run controller
- `cargo run --bin crd`: Output CRD
## Running

### Locally

```bash
# Create CRD
kubectl apply -f k8s/ats.yml
kubectl wait --for=condition=NamesAccepted crd/ats.example.kazk.dev

# Start controller
cargo run

# Create At resource example
kubectl apply -f k8s/example.yml
```

### Inside

```bash
# Set up service account
kubectl apply -f k8s/rbac.yml

# Create CRD
kubectl apply -f k8s/ats.yml
kubectl wait --for=condition=NamesAccepted crd/ats.example.kazk.dev

# Deploy controller
kubectl apply -f k8s/deployment.yml

# Create At resource example
kubectl apply -f k8s/example.yml
```
## Comparisons

### Binary Size

Building on Arch Linux:

- `client-go`: 31MB (23MB with `-ldflags "-s -w"`)
- `kubebuilder`: TBD requires unknown version of [KubeBuilder]
- `operator-sdk`: TBD requires unknown version of [Operator SDK]
- `kube`: 8.3MB (5.8MB with `RUSTFLAGS='-C link-arg=-s'`)

### Lines of Code

|   Library     |  Files   |  Lines  |   Code  |  Comments |  Blanks |
| :-----------: | -------: | ------: | ------: | --------: | ------: | 
| client-go     |    29    |   2161  |   1212  |       658 |     291 |
| kubebuilder   |    16    |   1013  |    529  |       357 |     127 |
| operator-sdk  |    13    |    714  |    488  |       134 |      92 |
| kube          |     5    |    257  |    217  |        15 |      25 |


- `client-go`: `tokei -e cnat-client-go/vendor -e cnat-client-go/hack -t Go cnat-client-go`
- `kubebuilder`: `tokei -e cnat-kubebuilder/vendor -e cnat-kubebuilder/hack -t Go cnat-kubebuilder`
- `operator-sdk`: `tokei -e cnat-operator/vendor -t Go cnat-operator`
- `kube`: `tokei -t Rust`

### Project Setup and Dependencies

- `client-go`: Go modules works fine (3 core modules + 1 code geerator + 1 logger). Versioning has improved since [programming-kubernetes/cnat] was written. A lot of generated code.
- `kubebuilder`: Requires [KubeBuilder] tool. Build failed with `GOPATH` not defined. Uses [dep].
- `operator-sdk`: Requires [Operator SDK] tool. Haven't tried. Uses [dep].
- `kube`: Standard Rust project. No separate codegen and very straightforward. CRD v1 schema is generated from the spec struct [clux/kube-rs#348] :)

## References

- [ConfigMapGenerator example](https://github.com/clux/kube-rs/blob/f49fcc4b64ca53091efe15f570e38c6ab3789567/examples/configmapgen_controller.rs)
- [at_controller.go](https://github.com/programming-kubernetes/cnat/blob/27f8ddba657b803ffb10501a28e003d0febd6387/cnat-kubebuilder/pkg/controller/at/at_controller.go)
- [h2oai/h2o-kubernetes](https://github.com/h2oai/h2o-kubernetes)


[`at`]: https://en.wikipedia.org/wiki/At_(command)
[programming-kubernetes/cnat]: https://github.com/programming-kubernetes/cnat
[KubeBuilder]: https://book.kubebuilder.io/quick-start.html
[Operator SDK]: https://github.com/operator-framework/operator-sdk
[kube]: https://github.com/clux/kube-rs
[clux/kube-rs#348]: https://github.com/clux/kube-rs/pull/348
[dep]: https://github.com/golang/dep
