import { defineRailway, github, project, service, volume } from "railway/iac";

export default defineRailway((ctx) => {
  const repository = process.env.HYPERMIND_SOURCE_REPO;
  if (!repository || !/^[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+$/.test(repository)) {
    throw new Error("Set HYPERMIND_SOURCE_REPO to the GitHub owner/repository to deploy");
  }
  const data = volume("hypermind-data", { region: "europe-west4", sizeMB: 10240 });
  const daemon = service("hypermind", {
    source: github(repository),
    build: { builder: "DOCKERFILE", dockerfilePath: "deploy/Dockerfile" },
    start: "/usr/local/bin/hm serve --config /var/lib/hypermind/hypermind.conf --init-if-missing --tls-from-env --grpc-bind [::]:8443 --rest-bind [::]:8444 --json",
    replicas: { "europe-west4": 1 },
    deploy: { restartPolicyType: "ON_FAILURE", restartPolicyMaxRetries: 3, overlapSeconds: 0, drainingSeconds: 60, requiredMountPath: "/var/lib/hypermind", healthcheckPath: null },
    tcp: [8443],
    volumeMounts: { "/var/lib/hypermind": data },
    env: {
      RAILWAY_RUN_UID: "0",
      HM_TLS_CERT_PEM: ctx.shared.HM_TLS_CERT_PEM,
      HM_TLS_KEY_PEM: ctx.shared.HM_TLS_KEY_PEM,
      HM_TLS_CLIENT_CA_PEM: ctx.shared.HM_TLS_CLIENT_CA_PEM,
    },
  });
  return project("hypermind", { resources: [daemon, data] });
});
