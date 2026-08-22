package main


func main() {
	acc := int64(1)
	const base int64 = 3
	const n int64 = 2500000
	for i := int64(0); i < n; i++ {
		acc = (acc * base) % 1000000007
	}
	suite5PrintI64(acc)
}
