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

/**
;;; No error occurred. System call completed successfully.
    $success 0
	;;; Permission denied.
	$bad_fd -1
    ;;; Argument list too long.
    $2big -2
    ;;; Permission denied.
    $acces -3
    ;;; Address in use.
    $addrinuse -4
    ;;; Address not available.
    $addrnotavail -5
    ;;; Address family not supported.
    $afnosupport -6
    ;;; Resource unavailable, or operation would block.
    $again -7
	;;; Bad file descriptor
    $badf -8
    ;;; Connection error
    $bad_connect
    ;;; Driver Not Register error
    $bad_driver
    ;;; Driver Open Error
    $bad_open
    ;;; Driver found bad params
    $bad_params
*/

type InnerContext struct {
	req  *http.Request
	resp *http.Response
}

var Context = make(map[int32]*InnerContext)

var MaxSeq int32 = 1

type Options struct {
	Method string `json:"method"`
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
	fmt.Println(string(opts_slice))
	if err := json.Unmarshal(opts_slice, &options); err != nil {
		fmt.Fprintf(os.Stderr, "error format params: %s\n", options)
		return -11
	}
	if req, err = http.NewRequest(options.Method, loc_url, nil); err != nil {
		fmt.Fprintf(os.Stderr, "new request error: %s\n", err)
		return -11
	} else {
		if resp, err = http.DefaultClient.Do(req); err != nil {
			fmt.Fprintf(os.Stderr, "do request error: %s\n", err)
			return -10
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
			return -1
		}
		fmt.Fprintf(os.Stderr, "read body error: %s\n", err)
		return -2
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
