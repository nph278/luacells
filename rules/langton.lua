-- Langton's Ant

Update = function(c, n)
  if c == 0 then
    if n[1] == 4 or n[1] == 9 then
      return 3
    elseif n[2] == 5 or n[2] == 8 then
      return 2
    elseif n[3] == 3 or n[3] == 6 then
      return 5
    elseif n[4] == 2 or n[4] == 7 then
      return 4
    end
    return 0
  elseif c == 1 then
    if n[1] == 4 or n[1] == 9 then
      return 7
    elseif n[2] == 5 or n[2] == 8 then
      return 6
    elseif n[3] == 3 or n[3] == 6 then
      return 9
    elseif n[4] == 2 or n[4] == 7 then
      return 8
    end
    return 1
  elseif c < 6 then
    return 1
  else
    return 0
  end
end

Display = function(n)
  if n == 0 then return "  " end
  if n == 1 then return "##" end
  if n == 2 then return " |" end
  if n == 3 then return " |" end
  if n == 4 then return " -" end
  if n == 5 then return " -" end
  if n == 6 then return "#|" end
  if n == 7 then return "#|" end
  if n == 8 then return "#-" end
  if n == 9 then return "#-" end
end

States = 10
