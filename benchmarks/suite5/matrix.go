package main


func main() {
	var a, b, c [4][4]int64
	for i := 0; i < 4; i++ {
		for j := 0; j < 4; j++ {
			a[i][j] = int64(i + j)
			b[i][j] = int64(i*j + 1)
			c[i][j] = 0
		}
	}
	const reps int64 = 900000
	var trace int64
	for r := int64(0); r < reps; r++ {
		for i := 0; i < 4; i++ {
			for j := 0; j < 4; j++ {
				var s int64
				for k := 0; k < 4; k++ {
					s += a[i][k] * b[k][j]
				}
				c[i][j] = s
			}
		}
		trace += c[r&3][r&3]
	}
	suite5PrintI64(trace)
}
