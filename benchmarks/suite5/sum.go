package main


func main() {
	const n int64 = 1500000
	var acc int64
	for i := int64(0); i < n; i++ {
		acc += i * i
	}
	suite5PrintI64(acc)
}
