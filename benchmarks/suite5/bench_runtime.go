package main

import (
	"fmt"
	"os"
)

var suite5BenchSink int64

func suite5PrintI64(n int64) {
	if os.Getenv("SUITE5_BENCH") != "" {
		suite5BenchSink = n
		return
	}
	fmt.Println(n)
}
