local M = {}

function M.debounce(ms, fn)
  vim.validate({
    ms = { ms, "number" },
    fn = { fn, "function" },
  })
  local timer_id = 0
  local pending_args = nil
  return function(...)
    timer_id = timer_id + 1
    local my_id = timer_id
    pending_args = { ... }
    vim.defer_fn(function()
      if my_id ~= timer_id then
        return
      end
      local args = pending_args
      pending_args = nil
      fn(unpack(args))
    end, ms)
  end
end

return M
