package main


func main() {
	const n int64 = 3000000
	var h int64
	var i int64
	for i = 0; i < n; i++ {
		h = (h*31 + i) % 1000000007
	}
	suite5PrintI64(h)
}
