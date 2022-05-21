package main

import "C"
import "unsafe"

//export blockless_open
func blockless_open(f *C.char, f_len int32, opts *C.char, o_len int32) int32 {
	return 1
}

//export blockless_read
func blockless_read(f int32, p *C.char, len int32) int32 {
	var slice = (*byte)(unsafe.Pointer(p))
	var bs = unsafe.Slice(slice, len)
	bs[0] = '1'
	bs[1] = '2'
	bs[2] = '3'
	return 3
}

//export blockless_write
func blockless_write(f int32, p *C.char, len int32) int32 {
	var slice = (*byte)(unsafe.Pointer(p))
	var bs = unsafe.Slice(slice, len)
	println(bs[0], bs[1], bs[2])
	return 3
}

func main() {

}
