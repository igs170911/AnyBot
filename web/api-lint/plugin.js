const METHODS_WITH_IDEMPOTENCY = ["post", "put", "patch", "delete"];

function IdempotencyKeyRequired() {
  return {
    PathItem: {
      Operation: {
        enter(operation, ctx) {
          const pointer = ctx.location.pointer;
          const method = (pointer.split("/").pop() || "").toLowerCase();
          if (!METHODS_WITH_IDEMPOTENCY.includes(method)) {
            return;
          }
          const parameters = operation.parameters || [];
          const hasKey = parameters.some(
            (parameter) =>
              parameter && parameter.name === "Idempotency-Key" && parameter.in === "header",
          );
          if (!hasKey) {
            ctx.report({
              message: `寫入操作 ${method} 缺少 Idempotency-Key 標頭參數（spec/05 §2）`,
              location: ctx.location.key("parameters"),
            });
          }
        },
      },
    },
  };
}

export default function anybotPlugin() {
  return {
    id: "anybot",
    rules: {
      oas3: {
        idempotencyKeyRequired: IdempotencyKeyRequired,
      },
    },
  };
}
