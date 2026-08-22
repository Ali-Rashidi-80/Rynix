package main


func main() {
	const n int64 = 2000000
	var acc int64
	var i int64
	for i = 0; i < n; i++ {
		acc = acc + i*3 - i/2 + i%7
	}
	suite5PrintI64(acc)
}
