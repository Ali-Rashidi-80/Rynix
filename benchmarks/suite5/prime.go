package main


func main() {
	const limit int64 = 100000
	var count int64
	for i := int64(2); i <= limit; i++ {
		prime := int64(1)
		for j := int64(2); j*j <= i; j++ {
			if i%j == 0 {
				prime = 0
				break
			}
		}
		if prime != 0 {
			count++
		}
	}
	suite5PrintI64(count)
}
