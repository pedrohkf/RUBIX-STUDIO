return table.freeze({
	GameName = "SampleGame",
	Version = "0.1.0",
	Ground = table.freeze({
		Size = Vector3.new(200, 1, 200),
		Color = Color3.fromRGB(96, 110, 96),
	}),
	Cube = table.freeze({
		Size = Vector3.new(4, 4, 4),
		Position = Vector3.new(0, 2, -20),
		Color = Color3.fromRGB(230, 90, 60),
		SpinSpeed = 90, -- graus por segundo
	}),
})
