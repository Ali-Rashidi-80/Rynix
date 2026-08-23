package main


func main() {
	acc := int64(1)
	const base int64 = 3
	n := suite5OpaqueI64(2500000)
	for i := int64(0); i < n; i++ {
		acc = (acc * base) % 1000000007
	}
	suite5PrintI64(acc)
}
