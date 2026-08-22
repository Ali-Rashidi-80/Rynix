package main


func gcd64(a, b int64) int64 {
	for b != 0 {
		a, b = b, a%b
	}
	return a
}

func main() {
	const n int64 = 2500000
	var acc int64
	for i := int64(1); i <= n; i++ {
		a := i * 9973
		b := i*1237 + 42
		acc += gcd64(a, b)
	}
	suite5PrintI64(acc)
}
