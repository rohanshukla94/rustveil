#include "llvm/ADT/STLExtras.h"
#include "llvm/IR/Constants.h"
#include "llvm/IR/IRBuilder.h"
#include "llvm/IR/InstIterator.h"
#include "llvm/IR/Instructions.h"
#include "llvm/IR/LLVMContext.h"
#include "llvm/IR/Module.h"
#include "llvm/IR/Verifier.h"
#include "llvm/IRReader/IRReader.h"
#include "llvm/Support/CommandLine.h"
#include "llvm/Support/FileSystem.h"
#include "llvm/Support/SourceMgr.h"
#include "llvm/Support/ToolOutputFile.h"
#include "llvm/Support/raw_ostream.h"

#include <cstdint>
#include <string>
#include <vector>

using namespace llvm;

static cl::opt<std::string> Input(cl::Positional, cl::Required,
                                  cl::desc("input LLVM IR"));
static cl::opt<std::string> Output("o", cl::Required,
                                   cl::desc("output LLVM IR (must not exist)"));
static cl::list<std::string> Functions("function", cl::OneOrMore,
                                      cl::desc("exact function symbol; repeat to select"));
static cl::opt<uint64_t> Seed("seed", cl::init(0),
                              cl::desc("deterministic mask seed; not a secret"));

static uint64_t nextMask(uint64_t &state) {
  state += UINT64_C(0x9e3779b97f4a7c15);
  uint64_t value = state;
  value = (value ^ (value >> 30)) * UINT64_C(0xbf58476d1ce4e5b9);
  value = (value ^ (value >> 27)) * UINT64_C(0x94d049bb133111eb);
  return value ^ (value >> 31);
}

static unsigned encodeConstants(Function &function, uint64_t &state) {
  std::vector<Instruction *> candidates;
  for (Instruction &instruction : instructions(function)) {
    if (isa<BinaryOperator>(instruction) || isa<ICmpInst>(instruction))
      candidates.push_back(&instruction);
  }

  unsigned changed = 0;
  for (Instruction *instruction : candidates) {
    for (unsigned index = 0; index < instruction->getNumOperands(); ++index) {
      auto *constant = dyn_cast<ConstantInt>(instruction->getOperand(index));
      if (!constant)
        continue;
      unsigned width = constant->getBitWidth();
      if (width != 8 && width != 16 && width != 32 && width != 64)
        continue;

      APInt mask(width, nextMask(state));
      while (mask.isZero() || mask == constant->getValue())
        mask = APInt(width, nextMask(state));

      auto *key = new GlobalVariable(
          *function.getParent(), constant->getType(), false,
          GlobalValue::PrivateLinkage, ConstantInt::get(constant->getType(), mask),
          "rv.key");
      IRBuilder<> builder(instruction);
      auto *loaded = builder.CreateLoad(constant->getType(), key, "rv.mask");
      // Volatile keeps the load even when the optimizer can see the initializer.
      loaded->setVolatile(true);
      auto *encoded = ConstantInt::get(constant->getType(), constant->getValue() ^ mask);
      instruction->setOperand(index, builder.CreateXor(loaded, encoded, "rv.value"));
      ++changed;
    }
  }
  return changed;
}

static void invalidateMemoryAttributes(Module &module) {
  // Callers may carry inferred memory(none), including at individual call sites.
  for (Function &function : module) {
    function.removeFnAttr(Attribute::Memory);
    function.removeFnAttr(Attribute::Speculatable);
    for (Instruction &instruction : instructions(function)) {
      if (auto *call = dyn_cast<CallBase>(&instruction)) {
        call->removeFnAttr(Attribute::Memory);
        call->removeFnAttr(Attribute::Speculatable);
      }
    }
  }
}

int main(int argc, char **argv) {
  cl::ParseCommandLineOptions(argc, argv, "RustVeil integer-constant experiment\n");
  if (sys::fs::exists(Output)) {
    errs() << "output already exists: " << Output << '\n';
    return 1;
  }

  LLVMContext context;
  SMDiagnostic diagnostic;
  auto module = parseIRFile(Input, diagnostic, context);
  if (!module) {
    diagnostic.print(argv[0], errs());
    return 1;
  }
  if (verifyModule(*module, &errs()))
    return 1;

  std::vector<Function *> selected;
  for (const std::string &name : Functions) {
    Function *function = module->getFunction(name);
    if (!function || function->isDeclaration()) {
      errs() << "function definition not found: " << name << '\n';
      return 1;
    }
    if (llvm::is_contained(selected, function)) {
      errs() << "duplicate function: " << name << '\n';
      return 1;
    }
    selected.push_back(function);
  }

  uint64_t state = Seed;
  unsigned changed = 0;
  for (Function *function : selected) {
    unsigned count = encodeConstants(*function, state);
    if (!count) {
      errs() << "no supported integer operands in: " << function->getName() << '\n';
      return 1;
    }
    changed += count;
  }
  invalidateMemoryAttributes(*module);
  if (verifyModule(*module, &errs()))
    return 1;

  std::error_code error;
  int descriptor;
  error = sys::fs::openFileForWrite(Output, descriptor, sys::fs::CD_CreateNew,
                                  sys::fs::OF_Text);
  if (error) {
    errs() << "cannot write output: " << error.message() << '\n';
    return 1;
  }
  ToolOutputFile output(Output, descriptor);
  module->print(output.os(), nullptr);
  output.os().flush();
  if (output.os().has_error()) {
    errs() << "failed to write output\n";
    return 1;
  }
  output.keep();
  outs() << "{\"format_version\":1,\"functions\":" << selected.size()
         << ",\"encoded_constants\":" << changed
         << ",\"seed\":\"" << Seed << "\"}\n";
}
