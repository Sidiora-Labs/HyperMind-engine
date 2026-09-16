package cortexclient

import (
	"context"
	"encoding/binary"
	"fmt"
	"hash/crc32"
	"io"
	"net"
	"time"

	"centra/core/cortexclient/wire/hypermind/protocol"
	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials"
	"google.golang.org/grpc/metadata"
)

type TransportError struct {
	EffectState string
	Cause       error
}

func (e *TransportError) Error() string {
	return fmt.Sprintf("hypermind transport effect_state=%s: %v", e.EffectState, e.Cause)
}
func (e *TransportError) Unwrap() error { return e.Cause }

type grpcMessage struct{ First, Second []byte }
type ncprCodec struct{}

func (ncprCodec) Name() string { return "proto" }
func (ncprCodec) Marshal(value any) ([]byte, error) {
	v, ok := value.(*grpcMessage)
	if !ok {
		return nil, ErrProtocol
	}
	var out []byte
	for index, payload := range [][]byte{v.First, v.Second} {
		if payload == nil {
			continue
		}
		out = append(out, byte((index+1)*8+2))
		out = binary.AppendUvarint(out, uint64(len(payload)))
		out = append(out, payload...)
	}
	return out, nil
}
func (ncprCodec) Unmarshal(data []byte, value any) error {
	v, ok := value.(*grpcMessage)
	if !ok {
		return ErrProtocol
	}
	*v = grpcMessage{}
	for len(data) > 0 {
		key, n := binary.Uvarint(data)
		if n <= 0 || key&7 != 2 {
			return ErrProtocol
		}
		data = data[n:]
		length, n := binary.Uvarint(data)
		if n <= 0 || length > uint64(len(data)-n) {
			return ErrProtocol
		}
		data = data[n:]
		payload := append([]byte(nil), data[:int(length)]...)
		data = data[int(length):]
		switch key >> 3 {
		case 1:
			v.First = payload
		case 2:
			v.Second = payload
		}
	}
	return nil
}

// grpcConn adapts the existing framed, sequenced client to the raw NCPR RPCs.
// The adapter never retries: only the donor sequence machine may replay writes.
type grpcConn struct {
	client                      *grpc.ClientConn
	ctx                         context.Context
	cancel                      context.CancelFunc
	hello                       []byte
	read                        []byte
	stream                      grpc.ClientStream
	readDeadline, writeDeadline time.Time
}

func dialGRPC(config Config) (net.Conn, error) {
	if config.TLSConfig == nil || config.TLSConfig.InsecureSkipVerify || config.TLSConfig.RootCAs == nil || len(config.TLSConfig.Certificates) == 0 {
		return nil, fmt.Errorf("%w: verified mutual TLS credentials are required", ErrCapability)
	}
	tlsConfig := config.TLSConfig.Clone()
	if tlsConfig.MinVersion < 0x0303 {
		tlsConfig.MinVersion = 0x0303
	}
	client, err := grpc.NewClient(config.GRPCTarget,
		grpc.WithTransportCredentials(credentials.NewTLS(tlsConfig)),
		grpc.WithDefaultCallOptions(grpc.ForceCodec(ncprCodec{}), grpc.MaxCallRecvMsgSize(maximumFrameBytes+4096), grpc.MaxCallSendMsgSize(maximumFrameBytes+4096)),
		grpc.WithDisableRetry())
	if err != nil {
		return nil, err
	}
	ctx, cancel := context.WithCancel(context.Background())
	return &grpcConn{client: client, ctx: ctx, cancel: cancel}, nil
}

func (c *grpcConn) Write(frame []byte) (int, error) {
	if len(frame) < 8 || int(getU32(frame)) != len(frame)-8 || crc32.Checksum(frame[8:], crc32cTable) != getU32(frame[4:]) {
		return 0, ErrProtocol
	}
	payload := frame[8:]
	ctx := c.ctx
	var cancel context.CancelFunc
	if !c.writeDeadline.IsZero() {
		ctx, cancel = context.WithDeadline(ctx, c.writeDeadline)
		defer cancel()
	}
	method := "Exchange"
	request := &grpcMessage{First: c.hello, Second: payload}
	if c.hello == nil {
		method = "Connect"
		request = &grpcMessage{First: payload}
	}
	envelope := protocol.GetRootAsWireEnvelope(payload, 0)
	if envelope.PayloadType() == protocol.WirePayloadRequest {
		var table = envelope.Table()
		if !envelope.Payload(&table) {
			return 0, ErrProtocol
		}
		var req protocol.Request
		req.Init(table.Bytes, table.Pos)
		if req.PayloadType() == protocol.RequestPayloadSubscribe {
			stream, err := c.client.NewStream(c.ctx, &grpc.StreamDesc{ServerStreams: true}, "/hypermind.v3.HyperMind/Subscribe")
			if err != nil {
				return 0, err
			}
			c.stream = stream
			if err = stream.SendMsg(request); err != nil {
				return 0, err
			}
			if err = stream.CloseSend(); err != nil {
				return 0, err
			}
			return len(frame), nil
		}
	}
	var response grpcMessage
	var trailer metadata.MD
	if err := c.client.Invoke(ctx, "/hypermind.v3.HyperMind/"+method, request, &response, grpc.Trailer(&trailer)); err != nil {
		state := "unknown"
		if values := trailer.Get("effect-state"); len(values) > 0 && values[0] == "not_dispatched" {
			state = "not_dispatched"
		}
		return 0, &TransportError{EffectState: state, Cause: err}
	}
	if method == "Connect" {
		c.hello = append([]byte(nil), payload...)
	}
	c.enqueue(response.First)
	return len(frame), nil
}
func (c *grpcConn) enqueue(payload []byte) {
	c.read = make([]byte, len(payload)+8)
	putU32(c.read, uint32(len(payload)))
	putU32(c.read[4:], crc32.Checksum(payload, crc32cTable))
	copy(c.read[8:], payload)
}
func (c *grpcConn) Read(out []byte) (int, error) {
	if len(c.read) == 0 && c.stream != nil {
		var response grpcMessage
		type received struct{ err error }
		result := make(chan received, 1)
		go func() { result <- received{c.stream.RecvMsg(&response)} }()
		var timer <-chan time.Time
		var deadline *time.Timer
		if !c.readDeadline.IsZero() {
			deadline = time.NewTimer(time.Until(c.readDeadline))
			timer = deadline.C
			defer deadline.Stop()
		}
		select {
		case value := <-result:
			if value.err != nil {
				return 0, value.err
			}
		case <-timer:
			c.cancel()
			return 0, context.DeadlineExceeded
		case <-c.ctx.Done():
			return 0, c.ctx.Err()
		}
		c.enqueue(response.First)
	}
	if len(c.read) == 0 {
		return 0, io.EOF
	}
	n := copy(out, c.read)
	c.read = c.read[n:]
	return n, nil
}
func (c *grpcConn) Close() error { c.cancel(); return c.client.Close() }

type grpcAddress string

func (a grpcAddress) Network() string    { return "grpc" }
func (a grpcAddress) String() string     { return string(a) }
func (c *grpcConn) LocalAddr() net.Addr  { return grpcAddress("client") }
func (c *grpcConn) RemoteAddr() net.Addr { return grpcAddress(c.client.Target()) }
func (c *grpcConn) SetDeadline(t time.Time) error {
	c.readDeadline = t
	c.writeDeadline = t
	return nil
}
func (c *grpcConn) SetReadDeadline(t time.Time) error  { c.readDeadline = t; return nil }
func (c *grpcConn) SetWriteDeadline(t time.Time) error { c.writeDeadline = t; return nil }

func effectState(state protocol.MutationEffectState) string {
	switch state {
	case protocol.MutationEffectStatenot_dispatched:
		return "not_dispatched"
	case protocol.MutationEffectStateunknown:
		return "unknown"
	case protocol.MutationEffectStaterejected:
		return "rejected"
	default:
		return "none"
	}
}
