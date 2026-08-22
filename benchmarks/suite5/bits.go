package main


func popcount64(x int64) int64 {
	var c int64
	for x != 0 {
		c += x & 1
		x >>= 1
	}
	return c
}

func main() {
	const n int64 = 25000000
	x := int64(1)
	var acc int64
	for i := int64(0); i < n; i++ {
		x = (x*31 + i) % 1000000007
		acc += popcount64(x)
	}
	suite5PrintI64(acc)
}
