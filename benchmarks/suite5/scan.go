package main


func main() {
	n := suite5OpaqueI64(8000000)
	var acc int64
	for i := int64(0); i < n; i++ {
		if i%3 == 0 || i%7 == 0 {
			acc++
		}
	}
	suite5PrintI64(acc)
}
