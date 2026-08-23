package main


func main() {
	n := suite5OpaqueI64(10000000)
	var acc int64
	for i := int64(0); i < n; i++ {
		acc = acc + i*31 - i/8 + i%13
	}
	suite5PrintI64(acc)
}
