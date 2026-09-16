package cortexclient

import (
	"context"
	"crypto/ecdsa"
	"crypto/elliptic"
	"crypto/rand"
	"crypto/tls"
	"crypto/x509"
	"crypto/x509/pkix"
	"encoding/pem"
	"errors"
	"fmt"
	"math/big"
	"net"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"testing"
	"time"
)

func testTLS(t *testing.T, directory string) (*tls.Config, string, string, string) {
	t.Helper()
	key, err := ecdsa.GenerateKey(elliptic.P256(), rand.Reader)
	if err != nil {
		t.Fatal(err)
	}
	ca := &x509.Certificate{SerialNumber: big.NewInt(1), Subject: pkix.Name{CommonName: "HyperMind SDK test CA"}, NotBefore: time.Now().Add(-time.Hour), NotAfter: time.Now().Add(time.Hour), IsCA: true, BasicConstraintsValid: true, KeyUsage: x509.KeyUsageCertSign | x509.KeyUsageDigitalSignature}
	caDER, err := x509.CreateCertificate(rand.Reader, ca, ca, &key.PublicKey, key)
	if err != nil {
		t.Fatal(err)
	}
	caPEM := pem.EncodeToMemory(&pem.Block{Type: "CERTIFICATE", Bytes: caDER})
	leaf := &x509.Certificate{SerialNumber: big.NewInt(2), Subject: pkix.Name{CommonName: "localhost"}, DNSNames: []string{"localhost"}, IPAddresses: []net.IP{net.ParseIP("127.0.0.1")}, NotBefore: ca.NotBefore, NotAfter: ca.NotAfter, ExtKeyUsage: []x509.ExtKeyUsage{x509.ExtKeyUsageServerAuth, x509.ExtKeyUsageClientAuth}, KeyUsage: x509.KeyUsageDigitalSignature}
	der, err := x509.CreateCertificate(rand.Reader, leaf, ca, &key.PublicKey, key)
	if err != nil {
		t.Fatal(err)
	}
	certPEM := pem.EncodeToMemory(&pem.Block{Type: "CERTIFICATE", Bytes: der})
	private, err := x509.MarshalPKCS8PrivateKey(key)
	if err != nil {
		t.Fatal(err)
	}
	keyPEM := pem.EncodeToMemory(&pem.Block{Type: "PRIVATE KEY", Bytes: private})
	caPath, certPath, keyPath := filepath.Join(directory, "ca.pem"), filepath.Join(directory, "cert.pem"), filepath.Join(directory, "key.pem")
	for p, b := range map[string][]byte{caPath: caPEM, certPath: certPEM, keyPath: keyPEM} {
		if err = os.WriteFile(p, b, 0600); err != nil {
			t.Fatal(err)
		}
	}
	identity, err := tls.X509KeyPair(certPEM, keyPEM)
	if err != nil {
		t.Fatal(err)
	}
	roots := x509.NewCertPool()
	roots.AppendCertsFromPEM(caPEM)
	return &tls.Config{RootCAs: roots, Certificates: []tls.Certificate{identity}, ServerName: "localhost", MinVersion: tls.VersionTLS12}, caPath, certPath, keyPath
}
func freeAddress(t *testing.T) string {
	t.Helper()
	listener, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatal(err)
	}
	address := listener.Addr().String()
	listener.Close()
	return address
}

func TestGRPCSessionAgainstRealDaemon(t *testing.T) {
	directory := t.TempDir()
	tlsConfig, ca, cert, key := testTLS(t, directory)
	address, adminAddress := freeAddress(t), freeAddress(t)
	socket := filepath.Join(directory, "hm.sock")
	configPath := filepath.Join(directory, "hm.conf")
	config := fmt.Sprintf("socket=%s\ndata=%s\nuser=%s\nkek=%s\nadmin_token=%s\nactor=11:%s\nprojection_map_bytes=67108864\n", socket, filepath.Join(directory, "data"), strings.Repeat("11", 16), strings.Repeat("22", 32), strings.Repeat("aa", 32), strings.Repeat("bb", 32))
	if err := os.WriteFile(configPath, []byte(config), 0600); err != nil {
		t.Fatal(err)
	}
	cmd := exec.Command(cortexdBinary(t), "serve", "--config", configPath, "--grpc-bind", address, "--grpc-admin-bind", adminAddress, "--tls-cert", cert, "--tls-key", key, "--tls-client-ca", ca)
	cmd.Stderr = os.Stderr
	cmd.Stdout = os.Stderr
	if err := cmd.Start(); err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { cmd.Process.Kill(); cmd.Wait() })
	waitForSocket(t, socket)
	cfg := Config{GRPCTarget: address, TLSConfig: tlsConfig, ConnectionID: connectionID(89), RequestTimeout: 5 * time.Second}
	for i := range cfg.CapabilityToken {
		cfg.CapabilityToken[i] = 0xbb
	}
	client, err := Dial(cfg)
	if err != nil {
		t.Fatal(err)
	}
	defer client.Close()
	session, err := client.Session("go-remote")
	if err != nil {
		t.Fatal(err)
	}
	ctx := context.Background()
	lsn, err := session.Remember(ctx, "cobalt lighthouse real remote memory")
	if err != nil {
		t.Fatal(err)
	}
	hits, err := session.Recall(ctx, "cobalt", RecallOptions{})
	if err != nil || len(hits) != 1 || hits[0] != lsn {
		t.Fatalf("recall %v %v", hits, err)
	}
	belief, err := session.Believe(ctx, map[string]any{"belief_id": "remote-fact", "belief_type": "fact", "canonical_identity": "remote-project", "value": "cobalt lighthouse", "provenance": []map[string]any{{"first_lsn": lsn, "last_lsn": lsn, "byte_start": 0, "byte_end": 6}}})
	if err != nil || belief["ok"] != true {
		t.Fatalf("believe %v %v", belief, err)
	}
	current, err := session.AsOf(ctx, 0, "remote-project", AsOfOptions{ValidAtNS: time.Now().UnixNano()})
	if err != nil || string(current.ValueBytes()) != "cobalt lighthouse" {
		t.Fatalf("as of %v %v", current, err)
	}
	bundle, err := session.Activate(ctx, "cobalt", 4096)
	if err != nil {
		t.Fatal(err)
	}
	if len(bundle.ModernSections) != 10 {
		t.Fatal("missing activation tiers")
	}
	for _, section := range session.Render(bundle, RenderOptions{SameTurnLSNs: []uint64{lsn}}).Sections {
		for _, item := range section.Items {
			if item.Role != "user" || item.Trust != "untrusted_memory" {
				t.Fatal("unsafe role")
			}
			for _, source := range item.Provenance {
				if source == lsn {
					t.Fatal("same turn leaked")
				}
			}
		}
	}
	if _, err = session.Remember(ctx, "RECONSTRUCTION inferred result"); err == nil {
		t.Fatal("reconstruction stored")
	}
	sub, err := client.Subscribe(ctx, &session.conversation, lsn)
	if err != nil {
		t.Fatal(err)
	}
	defer sub.Close()
	event, err := sub.Next(ctx)
	if err != nil || event.LSN <= lsn {
		t.Fatalf("replay %v %v", event, err)
	}
	if count, err := session.Attest(ctx, []uint64{lsn}, nil); err != nil || count != 1 {
		t.Fatalf("attest %d %v", count, err)
	}
	if _, err = client.CryptoDelete(ctx, 11); err == nil {
		t.Fatal("actor crypto delete accepted")
	}
	for _, verb := range []string{"remember", "recall", "activate", "attest", "believe", "retract", "dispute", "intend", "bind", "predict", "outcome", "consolidate", "forget", "inspect"} {
		envelope, err := client.CallTool(ctx, verb, map[string]any{})
		if err != nil {
			t.Fatalf("%s dispatch: %v", verb, err)
		}
		if _, ok := envelope["ok"].(bool); !ok {
			t.Fatalf("%s omitted error envelope", verb)
		}
	}
	next := client.Welcome().NextClientSeq
	client.Close()
	client, err = Dial(cfg)
	if err != nil {
		t.Fatal(err)
	}
	defer client.Close()
	if client.Welcome().NextClientSeq != next {
		t.Fatalf("sequence recovery got %d want %d", client.Welcome().NextClientSeq, next)
	}
	if _, err = client.Append(ctx, []AppendEvent{UserMsgEvent(session.conversation, "post reconnect")}); err != nil {
		t.Fatal(err)
	}
	adminCfg := cfg
	adminCfg.GRPCTarget = adminAddress
	for i := range adminCfg.CapabilityToken {
		adminCfg.CapabilityToken[i] = 0xaa
	}
	admin, err := Dial(adminCfg)
	if err != nil {
		t.Fatal(err)
	}
	defer admin.Close()
	if health, err := admin.AdminHealth(ctx); err != nil || !health.Ready {
		t.Fatalf("health %v %v", health, err)
	}
	bad := cfg
	bad.CapabilityToken = [32]byte{}
	if c, err := Dial(bad); err == nil {
		c.Close()
		t.Fatal("wrong capability accepted")
	}
	bad = cfg
	bad.TLSConfig = nil
	bad.DialTimeout = time.Millisecond
	if _, err := Dial(bad); err == nil || !errors.Is(err, ErrUnavailable) {
		t.Fatalf("missing TLS accepted: %v", err)
	}
	if receipt, err := admin.CryptoDelete(ctx, 11); err != nil || len(receipt) == 0 {
		t.Fatalf("admin crypto delete receipt=%d err=%v", len(receipt), err)
	}
}
