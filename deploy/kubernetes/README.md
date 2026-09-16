# Kubernetes and Helm

The chart at `deploy/helm/hypermind` runs one amd64 StatefulSet replica with a persistent volume, UID/GID 65532, a read-only root filesystem, no added capabilities, and no service-account token. It starts the real daemon, not a web health shim. Build and publish your own image; no published release tag is assumed:

```sh
docker build --platform linux/amd64 -t YOUR_REGISTRY/hypermind:YOUR_TAG -f deploy/Dockerfile .
docker push YOUR_REGISTRY/hypermind:YOUR_TAG
```

Resolve the pushed digest and use it in `image.digest`. Create an evaluation certificate with the service DNS name (production should use managed PKI):

```sh
deploy/tls/create-dev-certs.sh /absolute/new/private/tls-dir hypermind.hypermind.svc.cluster.local
kubectl create namespace hypermind
kubectl -n hypermind create secret generic hypermind-tls \
  --from-file=tls.crt=/absolute/new/private/tls-dir/server/server.pem \
  --from-file=tls.key=/absolute/new/private/tls-dir/server/server.key \
  --from-file=ca.crt=/absolute/new/private/tls-dir/server/client-ca.pem
helm upgrade --install hypermind deploy/helm/hypermind --namespace hypermind \
  --set image.repository=YOUR_REGISTRY/hypermind \
  --set image.digest=sha256:YOUR_PUSHED_DIGEST \
  --set existingTlsSecret=hypermind-tls --wait --timeout 10m
```

Only server credentials go in the Secret; do not mount the CA signing key or client private key in the daemon. Back up Secrets according to cluster policy. A new store receives private random keys during its init container. Existing configurations are validated and retained; the init container does not overwrite keys. The storage driver must honor `fsGroup=65532` or an operator must prepare the volume permissions before starting. Choose a real provisioner with durable storage using `persistence.storageClass`; do not share one actor directory among writers. [StatefulSets](https://kubernetes.io/docs/concepts/workloads/controllers/statefulset/), [security contexts](https://kubernetes.io/docs/tasks/configure-pod-container/security-context/)

Check authenticated readiness:

```sh
kubectl -n hypermind exec hypermind-0 -- /usr/local/bin/hm doctor \
  --config /var/lib/hypermind/hypermind.conf --require-healthy --json
```

Startup/readiness probes execute that same authenticated check; missing local ONNX models are reported separately and are not presented as configured inference. Optional provider variables come from an existing Secret selected with `existingProviderSecret` and remain runtime-only.

The default ClusterIP service exposes only actor gRPC 8443 and REST 8444 inside the cluster. Clients require the server CA, client cert/key, and the actor capability from the private config. Retrieve only its `actor=ID:TOKEN` entry using approved cluster/volume administration, never expose the full KEK-containing config or admin capability. The image has no `cat`, shell, or `tar`; do not assume shell-oriented `kubectl cp` workflows work.

For public access, set `service.type=LoadBalancer` only with a provider configuration that forwards **raw TCP** without terminating TLS. Provider-specific annotations may be supplied with `service.annotations`. Issue certificates for the actual external DNS SAN first. Ordinary HTTPS Ingress termination and unauthenticated HTTP probes are incompatible with this design; no Ingress is installed. [Kubernetes Services](https://kubernetes.io/docs/concepts/services-networking/service/)

Keep replicas at one. Before a restore or ownership migration, stop the StatefulSet and verify the writer has exited. Preserve the PVC and private configuration, and use encrypted backups of a quiescent volume. Helm rendering can be validated locally; this repository's local Docker checks do not certify a running Kubernetes cluster or its storage/load-balancer integration.
