package main


func main() {
	n := suite5OpaqueI64(5000000)
	var a, b int64 = 0, 1
	var i int64
	for i = 0; i < n; i++ {
		c := a + b
		a = b
		b = c
	}
	suite5PrintI64(a)
}
