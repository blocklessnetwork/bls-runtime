package main

import (
	"fmt"
	"runtime"
	"syscall"
)

//must be using tinygo for compile, example: tinygo build

//export call_test
func call_test() int32

//go:wasm-module blockless
//export blockless_open
func blockless_open(a string, fd *int) syscall.Errno

func main() {

	ch := make(chan int)
	fmt.Println("--------++++-sss-")
	go func() {
		var buf = make([]byte, 1024)
		var fd int
		if err := blockless_open("124.239.251.16:80", &fd); err != 0 {
			fmt.Println("err:", err)
			ch <- 12
			return
		}
		println("fd", fd)
		defer func() {
			syscall.Close(int(fd))
		}()
		for {
			var bs = []byte("GET / HTTP/1.1\r\n\r\n")
			if n, err := syscall.Write(int(fd), bs); err != nil {
				fmt.Println("w errr:", err)
				return
			} else if n == len(bs) {
				fmt.Println("w n", n)
				break
			}
		}

		fmt.Println("--go1 2--")
		for {
			if n, err := syscall.Read(int(fd), buf); err != nil {
				if err.(syscall.Errno) == syscall.EAGAIN {
					runtime.Gosched()
					continue
				}
				break
			} else if n == 0 {
				fmt.Println("000)")
				break
			} else {
				fmt.Println("em1111)")
				fmt.Println(string(buf))
			}
		}

		ch <- 12
	}()
	go func() {
		fmt.Println("Im go2 11111")
	}()
	s := <-ch
	fmt.Println("1", s)
}
