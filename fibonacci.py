def fibonacci(n):
    """Generate the first n Fibonacci numbers."""
    if n <= 0:
        return []
    if n == 1:
        return [0]

    fibs = [0, 1]
    for _ in range(2, n):
        fibs.append(fibs[-1] + fibs[-2])
    return fibs


if __name__ == "__main__":
    count = int(input("How many Fibonacci numbers? "))
    print(fibonacci(count))
