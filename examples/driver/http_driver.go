package main

import "C"
import (
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"os"
	"unsafe"
)

const ()

type InnerContext struct {
	req  *http.Request
	resp *http.Response
}

var Context = make(map[int32]*InnerContext)

var MaxSeq int32 = 1

type Options struct {
	Method         string `json:"method"`
	ConnectTimeout int32  `json:"connectTimeout"`
	ReadTimeout    int32  `json:"readTimeout"`
}

//export blockless_open
func blockless_open(f_ptr *C.char, f_len int32, opt_ptr *C.char, o_len int32, fd *int32) int32 {
	var slice = (*byte)(unsafe.Pointer(f_ptr))
	var url_slice = unsafe.Slice(slice, f_len)
	var loc_url = string(url_slice)
	var req *http.Request
	var resp *http.Response
	var err error

	slice = (*byte)(unsafe.Pointer(opt_ptr))
	var opts_slice = unsafe.Slice(slice, o_len)
	var options Options
	if err := json.Unmarshal(opts_slice, &options); err != nil {
		fmt.Fprintf(os.Stderr, "error format params: %s", string(opts_slice))
		return 11
	}
	if req, err = http.NewRequest(options.Method, loc_url, nil); err != nil {
		fmt.Fprintf(os.Stderr, "new request error: %s\n", err)
		return 11
	} else {
		if resp, err = http.DefaultClient.Do(req); err != nil {
			fmt.Fprintf(os.Stderr, "do request error: %s\n", err)
			return 10
		}
	}
	if len(Context) > 0 {
		MaxSeq++
	}
	Context[MaxSeq] = &InnerContext{req, resp}
	*fd = MaxSeq
	return 0
}

//export blockless_read
func blockless_read(fd int32, p *C.char, len int32, retn *int32) int32 {
	var slice = (*byte)(unsafe.Pointer(p))
	var bs = unsafe.Slice(slice, len)

	var ctx *InnerContext = Context[fd]
	if ctx == nil {
		return -8
	}
	if n, err := ctx.resp.Body.Read(bs); err != nil {
		if err == io.EOF {
			if n > 0 {
				*retn = int32(n)
			} else {
				*retn = int32(0)
			}
			return 0
		}
		fmt.Fprintf(os.Stderr, "read body error: %s\n", err)
		return 2
	} else {
		*retn = int32(n)
		return 0
	}
}

//export blockless_write
func blockless_write(fd int32, p *C.char, len int32, retn *int32) int32 {
	return -1
}

//export blockless_close
func blockless_close(fd int32) int32 {
	var ctx *InnerContext = Context[fd]
	if ctx == nil {
		return -8
	}

	if ctx.resp != nil {
		ctx.resp.Body.Close()
	}
	delete(Context, fd)
	return 0
}

func main() {

}
