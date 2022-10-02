Update = function(c, n)
  if c == 0 then
    if n[1] == 2 then
      return 2
    elseif n[3] ~= 0 and n[5] == 2 then
      return 2
    elseif n[4] ~= 0 and n[7] == 2 then
      return 2
    else
      return 0
    end
  elseif c == 1 then
    return 1
  else
    if n[1] == 2 then
      return 2
    elseif n[2] ~= 1 then
      return 0
    elseif n[6] == 0 then
      return 0
    elseif n[8] == 0 then
      return 0
    else
      return 2
    end
  end
end

Display = function(n)
  if n == 0 then return "  " end
  if n == 1 then return "//" end
  if n == 2 then return "**" end
end

States = 3
