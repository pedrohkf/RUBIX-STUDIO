-- Cor da barra de vida por faixa: verde, amarelo, laranja, vermelho.
local HealthColor = {}

function HealthColor.of(fraction: number): Color3
	if fraction >= 0.6 then
		return Color3.fromRGB(80, 200, 80)
	elseif fraction >= 0.3 then
		return Color3.fromRGB(230, 200, 60)
	elseif fraction >= 0.1 then
		return Color3.fromRGB(240, 140, 40)
	end
	return Color3.fromRGB(220, 50, 50)
end

return HealthColor
