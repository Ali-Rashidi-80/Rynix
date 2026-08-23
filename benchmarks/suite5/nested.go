package main


func main() {
	n := suite5OpaqueI64(450)
	var s int64
	var i int64
	for i = 0; i < n; i++ {
		var j int64
		for j = 0; j < n; j++ {
			s = s + (i*j+i)%97
		}
	}
	suite5PrintI64(s)
}
