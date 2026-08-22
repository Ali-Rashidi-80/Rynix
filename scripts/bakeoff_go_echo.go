// Comparable local loopback echo RPS (optional; requires Go).
// Same idea as rt/tests/load_harness.c — not a published claim.
package main

import (
	"fmt"
	"io"
	"net"
	"os"
	"time"
)

func main() {
	ln, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
	addr := ln.Addr().String()
	go func() {
		for {
			c, err := ln.Accept()
			if err != nil {
				return
			}
			go func(c net.Conn) {
				defer c.Close()
				buf := make([]byte, 64)
				n, _ := c.Read(buf)
				if n > 0 {
					_, _ = c.Write(buf[:n])
				}
			}(c)
		}
	}()

	const iters = 64
	t0 := time.Now()
	for i := 0; i < iters; i++ {
		c, err := net.Dial("tcp", addr)
		if err != nil {
			fmt.Fprintln(os.Stderr, err)
			os.Exit(1)
		}
		msg := []byte(fmt.Sprintf("L%d", i))
		if _, err := c.Write(msg); err != nil {
			fmt.Fprintln(os.Stderr, err)
			os.Exit(1)
		}
		buf := make([]byte, 64)
		n, err := io.ReadFull(c, buf[:len(msg)])
		c.Close()
		if err != nil || n != len(msg) {
			fmt.Fprintln(os.Stderr, "bad echo")
			os.Exit(2)
		}
	}
	sec := time.Since(t0).Seconds()
	if sec < 0.001 {
		sec = 0.001
	}
	fmt.Printf("bakeoff_go_echo ok  iters=%d  rps=%.1f\n", iters, float64(iters)/sec)
}
