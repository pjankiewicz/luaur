extern crate alloc;

#[cfg(test)]
mod dense_hash_tests;
pub mod enums;
pub mod functions;
#[cfg(test)]
mod insertion_ordered_map_tests;
pub mod macros;
pub mod methods;
pub mod records;
#[cfg(test)]
mod string_utils_tests;
pub mod type_aliases;
#[cfg(test)]
mod vec_deque_tests;

/// Minimal libc surface for wasm. On `wasm32-unknown-unknown` (no libc) every
/// shim is needed; on libc-bearing wasm (e.g. `wasm32-wasip1`, used to run the
/// suite on a 32-bit-pointer platform) the allocator shims are gated off inside
/// the module so they don't clash with wasi-libc's, while the functions wasi
/// lacks (mmap stubs, etc.) are still provided.
#[cfg(target_arch = "wasm32")]
pub mod wasm_libc;

/// Pure-Rust `strtod` shim for wasm (no libc on `wasm32-unknown-unknown`). The
/// scanning core is unit-tested natively, so the module is also compiled under
/// `test`; only the `#[no_mangle]` C entry point is wasm-gated.
#[cfg(any(target_arch = "wasm32", test))]
pub mod strtod_shim;

// C++ exposes this at namespace scope; codegen_assert! and friends reference
// `luaur_common::assert_call_handler` directly.
pub use functions::assert_call_handler::assert_call_handler;
pub use records::f_value::set_luau_bool_flags;

/// C++ CLI `setLuauFlagsDefault(value)` analog: set every non-Debug FFlag.
/// (Rust statics cannot self-register, so the list is generated explicitly.)
#[allow(non_snake_case)]
pub fn set_all_flags(value: bool) {
    FFlag::DesugaredArrayTypeReferenceIsEmpty.set(value);
    FFlag::FixMathNoisePrecision.set(value);
    FFlag::LuauAddRecursionCounterToNonStrictTypeChecker.set(value);
    FFlag::LuauAllowGlobalDeclarationToBeCalledClass.set(value);
    FFlag::LuauAlsoInstantiateInferredArguments.set(value);
    FFlag::LuauAutocompleteConst.set(value);
    FFlag::LuauAutocompleteExport.set(value);
    FFlag::LuauAutocompleteStringSingletonIntersection.set(value);
    FFlag::LuauBidirectionalInferenceBetterUnionHandling.set(value);
    FFlag::LuauCallFeedback.set(value);
    FFlag::LuauCheckFunctionStatementTypes.set(value);
    FFlag::LuauClosureUsageCounter.set(value);
    FFlag::LuauCodeGenCallWrapperEmitInst.set(value);
    FFlag::LuauCodegenBufferInteger.set(value);
    FFlag::LuauCodegenDsePtrStoreTagCheck.set(value);
    FFlag::LuauCodegenDseRestoreHints.set(value);
    FFlag::LuauCodegenExtraTableOpts.set(value);
    FFlag::LuauCodegenFixBufferLenCheck.set(value);
    FFlag::LuauCodegenForwardRematerialize.set(value);
    FFlag::LuauCodegenFreeBlocks.set(value);
    FFlag::LuauCodegenInteger2.set(value);
    FFlag::LuauCodegenIntegerArg3Fix.set(value);
    FFlag::LuauCodegenIntegerFastcall2k.set(value);
    FFlag::LuauCodegenLinearSetupEntryState3.set(value);
    FFlag::LuauCodegenLoadPropagateOrigin.set(value);
    FFlag::LuauCodegenNopPadding.set(value);
    FFlag::LuauCodegenProtectData.set(value);
    FFlag::LuauCodegenRecordAllBlockExitInfo.set(value);
    FFlag::LuauCodegenRegTag2.set(value);
    FFlag::LuauCodegenSuggestArgumentRegisterX64.set(value);
    FFlag::LuauCodegenVmExitSync.set(value);
    FFlag::LuauCodegenVmExitSyncFix.set(value);
    FFlag::LuauCompileDuptableConstantPack2.set(value);
    FFlag::LuauCompileFastcall3CostModel.set(value);
    FFlag::LuauCompileFoldOptimize.set(value);
    FFlag::LuauCompileInlineTableFunctions.set(value);
    FFlag::LuauCompileNewTableMutationTracker.set(value);
    FFlag::LuauCompileNoOptNext.set(value);
    FFlag::LuauCompilePropagateTableProps2.set(value);
    FFlag::LuauCompileStringInterpTargetTop.set(value);
    FFlag::LuauCompileTypeAliases.set(value);
    FFlag::LuauCompileUdataDirect.set(value);
    FFlag::LuauConcatDoesntAlwaysReturnString.set(value);
    FFlag::LuauConst2.set(value);
    FFlag::LuauConstJustReportErrorForUnderfill.set(value);
    FFlag::LuauConstraintGraph.set(value);
    FFlag::LuauCstExprGroup.set(value);
    FFlag::LuauCstTypeGroup.set(value);
    FFlag::LuauDirectFieldGet.set(value);
    FFlag::LuauDisallowRedefiningBuiltinTypes.set(value);
    FFlag::LuauEmitCallFeedback.set(value);
    FFlag::LuauErrorTolerantPrettyPrinting.set(value);
    FFlag::LuauExplicitTypeInstantiationSupport.set(value);
    // Experimental "export values" syntax is intentionally NOT enabled here: it
    // is incomplete in this port — a closure that captures an exported local
    // mis-compiles the upvalue register (the C++ reference handles it), so it can
    // produce out-of-range bytecode. Keep it off (default false) until the
    // export-table/closure codegen is fixed. Tests that exercise it set the flag
    // explicitly via a scoped override.
    FFlag::LuauExportValueSyntax.set(false);
    FFlag::LuauExportValueTypecheck.set(false);
    FFlag::LuauExternTypesNormalizeWithShapes.set(value);
    FFlag::LuauFixIndexerSubtypingOrdering.set(value);
    FFlag::LuauFixPropReadsOnMetatableTypes.set(value);
    FFlag::LuauInstantiateFunctionTypeBeforePush.set(value);
    FFlag::LuauInstantiateInSubtyping.set(value);
    FFlag::LuauInstantiationUsesPolarity.set(value);
    FFlag::LuauIntegerBufferFastcalls.set(value);
    FFlag::LuauIntegerFastcalls.set(value);
    FFlag::LuauIntegerLibrary.set(value);
    FFlag::LuauIntegerType2.set(value);
    FFlag::LuauIterativeInstantiationQueuer.set(value);
    FFlag::LuauKnowsTheDataModel3.set(value);
    FFlag::LuauLValueCompoundAssignmentVisitLhs.set(value);
    FFlag::LuauLimitUnificationRecursion.set(value);
    FFlag::LuauNativeCodeTargetCheck.set(value);
    FFlag::LuauNonStrictModeUseErrorSupressingTag.set(value);
    FFlag::LuauOccursCheckForAllBindings.set(value);
    FFlag::LuauPropagateFreeTypesIntoUnionAndIntersectionBounds.set(value);
    FFlag::LuauPropagateTypeAnnotationsInForInLoops.set(value);
    FFlag::LuauPropertyModifierMismatchErrors.set(value);
    FFlag::LuauReadOnlyIndexers.set(value);
    FFlag::LuauRefineNilFromTableIndexerResultType.set(value);
    FFlag::LuauRemoveConstraintSolverEmplace.set(value);
    FFlag::LuauReplacerIsSolverAgnostic.set(value);
    FFlag::LuauRequireResolveAliasNullCheck.set(value);
    FFlag::LuauResumeRestoreCcalls.set(value);
    FFlag::LuauSilenceDynamicFormatStringErrors.set(value);
    FFlag::LuauSolverV2.set(value);
    FFlag::LuauSubtypingMissingPropertiesAsNil.set(value);
    FFlag::LuauSubtypingTablesHasBetterErrorSuppression.set(value);
    FFlag::LuauTableEntriesDontNeedToMatchIndent.set(value);
    FFlag::LuauTableFreezeCheckIsSubtype.set(value);
    FFlag::LuauTidyTypePrototyping.set(value);
    FFlag::LuauTransitiveSubtyping.set(value);
    FFlag::LuauTweakAccessViolationReporting.set(value);
    FFlag::LuauTypeFunctionRobustness.set(value);
    FFlag::LuauTypeFunctionSerializeArgNames.set(value);
    FFlag::LuauTypeFunctionStructuredErrors.set(value);
    FFlag::LuauTypeFunctionSupportsFrozen.set(value);
    FFlag::LuauUdataDirectAccess6.set(value);
    FFlag::LuauUdtfTypeIsSubtypeOf.set(value);
    FFlag::LuauUseNativeStackGuard.set(value);
    FFlag::LuauVisitCallTypeArgsInDfg.set(value);
    FFlag::LuauYieldIter2.set(value);
}

/// FastFlag namespace `FFlag::` — static (non-dynamic) bool flags. Definitions
/// from `LUAU_FASTFLAGVARIABLE(...)` across this crate's sources are collected
/// here so C++ reads `FFlag::Name` map to `crate::FFlag::Name.get()`. (Rust
/// modules are not open like C++ namespaces, so the per-crate namespace module
/// is the aggregation point — see `crate::macros::luau_fastflagvariable`.)
#[allow(non_snake_case)]
pub mod FFlag {
    // CodeGen/src/IrRegAllocA64.cpp
    crate::LUAU_FASTFLAGVARIABLE!(DebugCodegenChaosA64);
    // CodeGen/src/CodeGen.cpp
    crate::LUAU_FASTFLAGVARIABLE!(DebugCodegenOptSize);
    // CodeGen/src/CodeGen.cpp
    crate::LUAU_FASTFLAGVARIABLE!(DebugCodegenSkipNumbering);
    // Analysis/src/FragmentAutocomplete.cpp
    crate::LUAU_FASTFLAGVARIABLE!(DebugLogFragmentsFromAutocomplete);
    // CodeGen/src/OptimizeConstProp.cpp
    crate::LUAU_FASTFLAGVARIABLE!(DebugLuauAbortingChecks);
    // Analysis/src/Frontend.cpp
    crate::LUAU_FASTFLAGVARIABLE!(DebugLuauAlwaysShowConstraintSolvingIncomplete);
    // Analysis/src/ConstraintSolver.cpp
    crate::LUAU_FASTFLAGVARIABLE!(DebugLuauAssertOnForcedConstraint);
    // Analysis/src/Normalize.cpp
    crate::LUAU_FASTFLAGVARIABLE!(DebugLuauCheckNormalizeInvariant);
    // Analysis/src/DumpCFG.cpp
    crate::LUAU_FASTFLAGVARIABLE!(DebugLuauDumpCFGJson);
    // Analysis/src/Frontend.cpp
    crate::LUAU_FASTFLAGVARIABLE!(DebugLuauForbidInternalTypes);
    // tests/Fixture.cpp
    crate::LUAU_FASTFLAGVARIABLE!(DebugLuauForceAllNewSolverTests);
    // tests/Fixture.cpp
    crate::LUAU_FASTFLAGVARIABLE!(DebugLuauForceAllOldSolverTests);
    // Analysis/src/Frontend.cpp
    crate::LUAU_FASTFLAGVARIABLE!(DebugLuauForceNonStrictMode);
    // Analysis/src/Frontend.cpp
    crate::LUAU_FASTFLAGVARIABLE!(DebugLuauForceOldSolver);
    // Analysis/src/Frontend.cpp
    crate::LUAU_FASTFLAGVARIABLE!(DebugLuauForceStrictMode);
    // Analysis/src/TypeArena.cpp
    crate::LUAU_FASTFLAGVARIABLE!(DebugLuauFreezeArena);
    // Analysis/src/TypeInfer.cpp
    crate::LUAU_FASTFLAGVARIABLE!(DebugLuauFreezeDuringUnification);
    // Analysis/src/ConstraintSolver.cpp
    crate::LUAU_FASTFLAGVARIABLE!(DebugLuauLogBindings);
    // Analysis/src/DumpCFG.cpp
    crate::LUAU_FASTFLAGVARIABLE!(DebugLuauLogCFG);
    // Analysis/src/ConstraintSolver.cpp
    crate::LUAU_FASTFLAGVARIABLE!(DebugLuauLogSolver);
    // Analysis/src/Frontend.cpp
    crate::LUAU_FASTFLAGVARIABLE!(DebugLuauLogSolverToJson);
    // Analysis/src/Frontend.cpp
    crate::LUAU_FASTFLAGVARIABLE!(DebugLuauLogSolverToJsonFile);
    // Analysis/src/TypeFunction.cpp
    crate::LUAU_FASTFLAGVARIABLE!(DebugLuauLogTypeFamilies);
    // Analysis/src/TypeInfer.cpp
    crate::LUAU_FASTFLAGVARIABLE!(DebugLuauMagicTypes);
    // Analysis/src/AutocompleteCore.cpp
    crate::LUAU_FASTFLAGVARIABLE!(DebugLuauMagicVariableNames);
    // Ast/src/Parser.cpp
    crate::LUAU_FASTFLAGVARIABLE!(DebugLuauNoInline);
    // Analysis/src/Subtyping.cpp
    crate::LUAU_FASTFLAGVARIABLE!(DebugLuauSubtypingCheckPathValidity);
    // Common/src/TimeTrace.cpp
    crate::LUAU_FASTFLAGVARIABLE!(DebugLuauTimeTracing);
    // Analysis/src/ToString.cpp
    crate::LUAU_FASTFLAGVARIABLE!(DebugLuauToStringNoLexicalSort);
    // Ast/src/Parser.cpp
    crate::LUAU_FASTFLAGVARIABLE!(DebugLuauUserDefinedClasses);
    // VM/src/lvmexecute.cpp
    crate::LUAU_FASTFLAGVARIABLE!(DebugLuauUserDefinedClassesRuntime);
    // Ast/src/Parser.cpp
    crate::LUAU_FASTFLAGVARIABLE!(DesugaredArrayTypeReferenceIsEmpty);
    // VM/src/lmathlib.cpp
    crate::LUAU_FASTFLAGVARIABLE!(FixMathNoisePrecision);
    // Analysis/src/NonStrictTypeChecker.cpp
    crate::LUAU_FASTFLAGVARIABLE!(LuauAddRecursionCounterToNonStrictTypeChecker);
    // Ast/src/Parser.cpp
    crate::LUAU_FASTFLAGVARIABLE!(LuauAllowGlobalDeclarationToBeCalledClass);
    // Analysis/src/ConstraintSolver.cpp
    crate::LUAU_FASTFLAGVARIABLE!(LuauAlsoInstantiateInferredArguments);
    // Analysis/src/AutocompleteCore.cpp
    crate::LUAU_FASTFLAGVARIABLE!(LuauAutocompleteConst);
    // Analysis/src/AutocompleteCore.cpp
    crate::LUAU_FASTFLAGVARIABLE!(LuauAutocompleteExport);
    // Analysis/src/AutocompleteCore.cpp
    crate::LUAU_FASTFLAGVARIABLE!(LuauAutocompleteStringSingletonIntersection);
    // Analysis/src/ExpectedTypeVisitor.cpp
    crate::LUAU_FASTFLAGVARIABLE!(LuauBidirectionalInferenceBetterUnionHandling);
    // VM/src/lvmexecute.cpp
    crate::LUAU_FASTFLAGVARIABLE!(LuauCallFeedback);
    // Analysis/src/TypeChecker2.cpp
    crate::LUAU_FASTFLAGVARIABLE!(LuauCheckFunctionStatementTypes);
    // VM/src/lvmexecute.cpp
    crate::LUAU_FASTFLAGVARIABLE!(LuauClosureUsageCounter);
    // CodeGen/src/EmitInstructionX64.cpp
    crate::LUAU_FASTFLAGVARIABLE!(LuauCodeGenCallWrapperEmitInst);
    // CodeGen/src/IrTranslateBuiltins.cpp
    crate::LUAU_FASTFLAGVARIABLE!(LuauCodegenBufferInteger);
    // CodeGen/src/OptimizeDeadStore.cpp
    crate::LUAU_FASTFLAGVARIABLE!(LuauCodegenDsePtrStoreTagCheck);
    // CodeGen/src/OptimizeDeadStore.cpp
    crate::LUAU_FASTFLAGVARIABLE!(LuauCodegenDseRestoreHints);
    // CodeGen/src/OptimizeConstProp.cpp
    crate::LUAU_FASTFLAGVARIABLE!(LuauCodegenExtraTableOpts);
    // CodeGen/src/IrLoweringA64.cpp
    crate::LUAU_FASTFLAGVARIABLE!(LuauCodegenFixBufferLenCheck);
    // CodeGen/src/IrValueLocationTracking.cpp
    crate::LUAU_FASTFLAGVARIABLE!(LuauCodegenForwardRematerialize);
    // CodeGen/src/CodeAllocator.cpp
    crate::LUAU_FASTFLAGVARIABLE!(LuauCodegenFreeBlocks);
    // CodeGen/src/CodeGen.cpp
    crate::LUAU_FASTFLAGVARIABLE!(LuauCodegenInteger2);
    // CodeGen/src/IrTranslateBuiltins.cpp
    crate::LUAU_FASTFLAGVARIABLE!(LuauCodegenIntegerArg3Fix);
    // CodeGen/src/IrTranslation.cpp
    crate::LUAU_FASTFLAGVARIABLE!(LuauCodegenIntegerFastcall2k);
    // CodeGen/src/OptimizeConstProp.cpp
    crate::LUAU_FASTFLAGVARIABLE!(LuauCodegenLinearSetupEntryState3);
    // CodeGen/src/OptimizeConstProp.cpp
    crate::LUAU_FASTFLAGVARIABLE!(LuauCodegenLoadPropagateOrigin);
    // CodeGen/src/CodeGen.cpp
    crate::LUAU_FASTFLAGVARIABLE!(LuauCodegenNopPadding);
    // CodeGen/src/CodeAllocator.cpp
    crate::LUAU_FASTFLAGVARIABLE!(LuauCodegenProtectData);
    // CodeGen/src/OptimizeConstProp.cpp
    crate::LUAU_FASTFLAGVARIABLE!(LuauCodegenRecordAllBlockExitInfo);
    // CodeGen/src/BytecodeAnalysis.cpp
    crate::LUAU_FASTFLAGVARIABLE!(LuauCodegenRegTag2);
    // CodeGen/src/CodeGenX64.cpp
    crate::LUAU_FASTFLAGVARIABLE!(LuauCodegenSuggestArgumentRegisterX64);
    // CodeGen/src/IrAnalysis.cpp
    crate::LUAU_FASTFLAGVARIABLE!(LuauCodegenVmExitSync);
    // CodeGen/src/OptimizeDeadStore.cpp
    crate::LUAU_FASTFLAGVARIABLE!(LuauCodegenVmExitSyncFix);
    // Compiler/src/Compiler.cpp
    crate::LUAU_FASTFLAGVARIABLE!(LuauCompileDuptableConstantPack2);
    // Compiler/src/CostModel.cpp
    crate::LUAU_FASTFLAGVARIABLE!(LuauCompileFastcall3CostModel);
    // Compiler/src/ConstantFolding.cpp
    crate::LUAU_FASTFLAGVARIABLE!(LuauCompileFoldOptimize);
    // Compiler/src/Compiler.cpp
    crate::LUAU_FASTFLAGVARIABLE!(LuauCompileInlineTableFunctions);
    // Compiler/src/ConstantFolding.cpp
    crate::LUAU_FASTFLAGVARIABLE!(LuauCompileNewTableMutationTracker);
    // Compiler/src/Compiler.cpp
    crate::LUAU_FASTFLAGVARIABLE!(LuauCompileNoOptNext);
    // Compiler/src/ConstantFolding.cpp
    crate::LUAU_FASTFLAGVARIABLE!(LuauCompilePropagateTableProps2);
    // Compiler/src/Compiler.cpp
    crate::LUAU_FASTFLAGVARIABLE!(LuauCompileStringInterpTargetTop);
    // Compiler/src/Types.cpp
    crate::LUAU_FASTFLAGVARIABLE!(LuauCompileTypeAliases);
    // Bytecode/src/BytecodeBuilder.cpp
    crate::LUAU_FASTFLAGVARIABLE!(LuauCompileUdataDirect);
    // Analysis/src/BuiltinTypeFunctions.cpp
    crate::LUAU_FASTFLAGVARIABLE!(LuauConcatDoesntAlwaysReturnString);
    // Ast/src/Parser.cpp
    crate::LUAU_FASTFLAGVARIABLE!(LuauConst2);
    // Ast/src/Parser.cpp
    crate::LUAU_FASTFLAGVARIABLE!(LuauConstJustReportErrorForUnderfill);
    // Analysis/src/Constraint.cpp
    crate::LUAU_FASTFLAGVARIABLE!(LuauConstraintGraph);
    // Ast/src/Parser.cpp
    crate::LUAU_FASTFLAGVARIABLE!(LuauCstExprGroup);
    // Ast/src/Parser.cpp
    crate::LUAU_FASTFLAGVARIABLE!(LuauCstTypeGroup);
    // VM/src/lvmexecute.cpp
    crate::LUAU_FASTFLAGVARIABLE!(LuauDirectFieldGet);
    // Analysis/src/ConstraintGenerator.cpp
    crate::LUAU_FASTFLAGVARIABLE!(LuauDisallowRedefiningBuiltinTypes);
    // Compiler/src/Compiler.cpp
    crate::LUAU_FASTFLAGVARIABLE!(LuauEmitCallFeedback);
    // Ast/src/PrettyPrinter.cpp
    crate::LUAU_FASTFLAGVARIABLE!(LuauErrorTolerantPrettyPrinting);
    // Analysis/src/TypeInfer.cpp
    crate::LUAU_FASTFLAGVARIABLE!(LuauExplicitTypeInstantiationSupport);
    // Ast/src/Parser.cpp
    crate::LUAU_FASTFLAGVARIABLE!(LuauExportValueSyntax);
    // Analysis/src/Frontend.cpp
    crate::LUAU_FASTFLAGVARIABLE!(LuauExportValueTypecheck);
    // Analysis/src/Normalize.cpp
    crate::LUAU_FASTFLAGVARIABLE!(LuauExternTypesNormalizeWithShapes);
    // Analysis/src/Unifier.cpp
    crate::LUAU_FASTFLAGVARIABLE!(LuauFixIndexerSubtypingOrdering);
    // Analysis/src/ConstraintSolver.cpp
    crate::LUAU_FASTFLAGVARIABLE!(LuauFixPropReadsOnMetatableTypes);
    // Analysis/src/ConstraintSolver.cpp
    crate::LUAU_FASTFLAGVARIABLE!(LuauInstantiateFunctionTypeBeforePush);
    // Analysis/src/Unifier.cpp
    crate::LUAU_FASTFLAGVARIABLE!(LuauInstantiateInSubtyping);
    // Analysis/src/Instantiation.cpp
    crate::LUAU_FASTFLAGVARIABLE!(LuauInstantiationUsesPolarity);
    // Compiler/src/Builtins.cpp
    crate::LUAU_FASTFLAGVARIABLE!(LuauIntegerBufferFastcalls);
    // Compiler/src/Builtins.cpp
    crate::LUAU_FASTFLAGVARIABLE!(LuauIntegerFastcalls);
    // VM/src/lintlib.cpp
    crate::LUAU_FASTFLAGVARIABLE!(LuauIntegerLibrary);
    // Ast/src/Parser.cpp
    crate::LUAU_FASTFLAGVARIABLE!(LuauIntegerType2);
    // Analysis/src/ConstraintSolver.cpp
    crate::LUAU_FASTFLAGVARIABLE!(LuauIterativeInstantiationQueuer);
    // Analysis/src/Frontend.cpp
    crate::LUAU_FASTFLAGVARIABLE!(LuauKnowsTheDataModel3);
    // Analysis/src/TypeChecker2.cpp
    crate::LUAU_FASTFLAGVARIABLE!(LuauLValueCompoundAssignmentVisitLhs);
    // Analysis/src/Unifier2.cpp
    crate::LUAU_FASTFLAGVARIABLE!(LuauLimitUnificationRecursion);
    // CodeGen/src/CodeGenUtils.cpp
    crate::LUAU_FASTFLAGVARIABLE!(LuauNativeCodeTargetCheck);
    // Analysis/src/NonStrictTypeChecker.cpp
    crate::LUAU_FASTFLAGVARIABLE!(LuauNonStrictModeUseErrorSupressingTag);
    // Analysis/src/ConstraintSolver.cpp
    crate::LUAU_FASTFLAGVARIABLE!(LuauOccursCheckForAllBindings);
    // Analysis/src/Unifier2.cpp
    crate::LUAU_FASTFLAGVARIABLE!(LuauPropagateFreeTypesIntoUnionAndIntersectionBounds);
    // Analysis/src/ConstraintGenerator.cpp
    crate::LUAU_FASTFLAGVARIABLE!(LuauPropagateTypeAnnotationsInForInLoops);
    // Analysis/src/TypeChecker2.cpp
    crate::LUAU_FASTFLAGVARIABLE!(LuauPropertyModifierMismatchErrors);
    // Analysis/src/ConstraintGenerator.cpp
    crate::LUAU_FASTFLAGVARIABLE!(LuauReadOnlyIndexers);
    // Analysis/src/ConstraintSolver.cpp
    crate::LUAU_FASTFLAGVARIABLE!(LuauRefineNilFromTableIndexerResultType);
    // Analysis/src/ConstraintSolver.cpp
    crate::LUAU_FASTFLAGVARIABLE!(LuauRemoveConstraintSolverEmplace);
    // Analysis/src/Instantiation.cpp
    crate::LUAU_FASTFLAGVARIABLE!(LuauReplacerIsSolverAgnostic);
    // Require/src/RequireNavigator.cpp
    crate::LUAU_FASTFLAGVARIABLE!(LuauRequireResolveAliasNullCheck);
    // VM/src/ldo.cpp
    crate::LUAU_FASTFLAGVARIABLE!(LuauResumeRestoreCcalls);
    // Analysis/src/BuiltinDefinitions.cpp
    crate::LUAU_FASTFLAGVARIABLE!(LuauSilenceDynamicFormatStringErrors);
    // Ast/src/Parser.cpp
    crate::LUAU_FASTFLAGVARIABLE!(LuauSolverV2);
    // Analysis/src/Subtyping.cpp
    crate::LUAU_FASTFLAGVARIABLE!(LuauSubtypingMissingPropertiesAsNil);
    // Analysis/src/Subtyping.cpp
    crate::LUAU_FASTFLAGVARIABLE!(LuauSubtypingTablesHasBetterErrorSuppression);
    // Ast/src/Parser.cpp
    crate::LUAU_FASTFLAGVARIABLE!(LuauTableEntriesDontNeedToMatchIndent);
    // Analysis/src/BuiltinDefinitions.cpp
    crate::LUAU_FASTFLAGVARIABLE!(LuauTableFreezeCheckIsSubtype);
    // Analysis/src/ConstraintGenerator.cpp
    crate::LUAU_FASTFLAGVARIABLE!(LuauTidyTypePrototyping);
    // Analysis/src/Unifier.cpp
    crate::LUAU_FASTFLAGVARIABLE!(LuauTransitiveSubtyping);
    // Analysis/src/Error.cpp
    crate::LUAU_FASTFLAGVARIABLE!(LuauTweakAccessViolationReporting);
    // Analysis/src/TypeFunctionRuntime.cpp
    crate::LUAU_FASTFLAGVARIABLE!(LuauTypeFunctionRobustness);
    // Analysis/src/TypeFunctionRuntime.cpp
    crate::LUAU_FASTFLAGVARIABLE!(LuauTypeFunctionSerializeArgNames);
    // Analysis/src/TypeFunctionRuntime.cpp
    crate::LUAU_FASTFLAGVARIABLE!(LuauTypeFunctionStructuredErrors);
    // Analysis/src/TypeFunctionRuntime.cpp
    crate::LUAU_FASTFLAGVARIABLE!(LuauTypeFunctionSupportsFrozen);
    // VM/src/lvmload.cpp
    crate::LUAU_FASTFLAGVARIABLE!(LuauUdataDirectAccess6);
    // Analysis/src/TypeFunctionRuntime.cpp
    crate::LUAU_FASTFLAGVARIABLE!(LuauUdtfTypeIsSubtypeOf);
    // Analysis/src/NativeStackGuard.cpp
    crate::LUAU_FASTFLAGVARIABLE!(LuauUseNativeStackGuard);
    // Analysis/src/DataFlowGraph.cpp
    crate::LUAU_FASTFLAGVARIABLE!(LuauVisitCallTypeArgsInDfg);
    // VM/src/lvmexecute.cpp
    crate::LUAU_FASTFLAGVARIABLE!(LuauYieldIter2);
}

/// Static int FastFlags, mirroring `FFlag`. C++ collects every
/// `LUAU_FASTINTVARIABLE(...)` into `namespace FInt`; Rust modules aren't open,
/// so the consumers' flags are gathered here. Read as `FInt::Flag.get()`.
#[allow(non_snake_case)]
pub mod FInt {
    // CodeGen/src/CodeGen.cpp
    crate::LUAU_FASTINTVARIABLE!(CodegenHeuristicsBlockInstructionLimit, 65_536);
    // CodeGen/src/CodeGen.cpp
    crate::LUAU_FASTINTVARIABLE!(CodegenHeuristicsBlockLimit, 32_768);
    // CodeGen/src/CodeGen.cpp
    crate::LUAU_FASTINTVARIABLE!(CodegenHeuristicsInstructionLimit, 1_048_576);
    // CodeGen/src/CodeGenContext.cpp
    crate::LUAU_FASTINTVARIABLE!(LuauCodeGenBlockSize, 4 * 1024 * 1024);
    // CodeGen/src/CodeGenContext.cpp
    crate::LUAU_FASTINTVARIABLE!(LuauCodeGenMaxTotalSize, 256 * 1024 * 1024);
    // Analysis/src/Clone.cpp
    crate::LUAU_FASTINTVARIABLE!(LuauTypeCloneIterationLimit, 100_000);
    // Analysis/src/ToString.cpp
    crate::LUAU_FASTINTVARIABLE!(DebugLuauVerboseTypeNames, 0);
    // Analysis/src/TypeInfer.cpp
    crate::LUAU_FASTINTVARIABLE!(LuauCheckRecursionLimit, 300);
    // CodeGen/src/OptimizeConstProp.cpp
    crate::LUAU_FASTINTVARIABLE!(LuauCodeGenLiveSlotReuseLimit, 8);
    // CodeGen/src/OptimizeConstProp.cpp
    crate::LUAU_FASTINTVARIABLE!(LuauCodeGenMinLinearBlockPath, 3);
    // CodeGen/src/OptimizeConstProp.cpp
    crate::LUAU_FASTINTVARIABLE!(LuauCodeGenReuseSlotLimit, 64);
    // CodeGen/src/OptimizeConstProp.cpp
    crate::LUAU_FASTINTVARIABLE!(LuauCodeGenReuseUdataTagLimit, 64);
    // Compiler/src/Compiler.cpp
    crate::LUAU_FASTINTVARIABLE!(LuauCompileInlineDepth, 5);
    // Compiler/src/Compiler.cpp
    crate::LUAU_FASTINTVARIABLE!(LuauCompileInlineThreshold, 25);
    // Compiler/src/Compiler.cpp
    crate::LUAU_FASTINTVARIABLE!(LuauCompileInlineThresholdMaxBoost, 300);
    // Compiler/src/Compiler.cpp
    crate::LUAU_FASTINTVARIABLE!(LuauCompileLoopUnrollThreshold, 25);
    // Compiler/src/Compiler.cpp
    crate::LUAU_FASTINTVARIABLE!(LuauCompileLoopUnrollThresholdMaxBoost, 300);
    // Analysis/src/Generalization.cpp
    crate::LUAU_FASTINTVARIABLE!(LuauGenericCounterMaxDepth, 15);
    // Analysis/src/Generalization.cpp
    crate::LUAU_FASTINTVARIABLE!(LuauGenericCounterMaxSteps, 1500);
    // Analysis/src/Error.cpp
    crate::LUAU_FASTINTVARIABLE!(LuauIndentTypeMismatchMaxTypeLength, 10);
    // VM/src/lfunc.cpp
    crate::LUAU_FASTINTVARIABLE!(LuauInlineHitsThreshold, 3);
    // Analysis/src/NonStrictTypeChecker.cpp
    crate::LUAU_FASTINTVARIABLE!(LuauNonStrictTypeCheckerRecursionLimit, 300);
    // Analysis/src/Normalize.cpp
    crate::LUAU_FASTINTVARIABLE!(LuauNormalizeCacheLimit, 100000);
    // Analysis/src/Normalize.cpp
    crate::LUAU_FASTINTVARIABLE!(LuauNormalizerInitialFuel, 3000);
    // Ast/src/Parser.cpp
    crate::LUAU_FASTINTVARIABLE!(LuauParseErrorLimit, 100);
    // Analysis/src/ConstraintGenerator.cpp
    crate::LUAU_FASTINTVARIABLE!(LuauPrimitiveInferenceInTableLimit, 500);
    // Ast/src/Parser.cpp
    crate::LUAU_FASTINTVARIABLE!(LuauRecursionLimit, 1000);
    // Analysis/src/ConstraintSolver.cpp
    crate::LUAU_FASTINTVARIABLE!(LuauSolverConstraintLimit, 1000);
    // Analysis/src/ConstraintSolver.cpp
    crate::LUAU_FASTINTVARIABLE!(LuauSolverRecursionLimit, 500);
    // Analysis/src/NativeStackGuard.cpp
    crate::LUAU_FASTINTVARIABLE!(LuauStackGuardThreshold, 1024);
    // Analysis/src/Subtyping.cpp
    crate::LUAU_FASTINTVARIABLE!(LuauSubtypingIterationLimit, 20000);
    // Analysis/src/Subtyping.cpp
    crate::LUAU_FASTINTVARIABLE!(LuauSubtypingReasoningLimit, 100);
    // Analysis/src/Linter.cpp
    crate::LUAU_FASTINTVARIABLE!(LuauSuggestionDistance, 4);
    // Analysis/src/Type.cpp
    crate::LUAU_FASTINTVARIABLE!(LuauTableTypeMaximumStringifierLength, 0);
    // Analysis/src/Substitution.cpp
    crate::LUAU_FASTINTVARIABLE!(LuauTarjanChildLimit, 10000);
    // Analysis/src/Substitution.cpp
    crate::LUAU_FASTINTVARIABLE!(LuauTarjanPreallocationSize, 256);
    // Analysis/src/TypeInfer.cpp
    crate::LUAU_FASTINTVARIABLE!(LuauTypeInferIterationLimit, 20000);
    // Analysis/src/TypeInfer.cpp
    crate::LUAU_FASTINTVARIABLE!(LuauTypeInferRecursionLimit, 165);
    // Analysis/src/TypeInfer.cpp
    crate::LUAU_FASTINTVARIABLE!(LuauTypeInferTypePackLoopLimit, 5000);
    // Ast/src/Parser.cpp
    crate::LUAU_FASTINTVARIABLE!(LuauTypeLengthLimit, 1000);
    // Analysis/src/Type.cpp
    crate::LUAU_FASTINTVARIABLE!(LuauTypeMaximumStringifierLength, 500);
    // Analysis/src/TypeInfer.cpp
    crate::LUAU_FASTINTVARIABLE!(LuauVisitRecursionLimit, 500);
}

/// Dynamic bool flags (`DFFlag::`), mirroring `FFlag`.
#[allow(non_snake_case)]
pub mod DFFlag {
    // CodeGen/src/EmitCommonX64.cpp
    crate::LUAU_DYNAMIC_FASTFLAGVARIABLE!(AddReturnExectargetCheck, false);
    // Ast/src/Parser.cpp
    crate::LUAU_DYNAMIC_FASTFLAGVARIABLE!(DebugLuauReportReturnTypeVariadicWithTypeSuffix, false);
    // Require/src/RequireNavigator.cpp
    crate::LUAU_DYNAMIC_FASTFLAGVARIABLE!(LuauRequireAliasOverrideOrderFix, false);
}

/// Dynamic int flags (`DFInt::`), mirroring `FInt`.
#[allow(non_snake_case)]
pub mod DFInt {
    // Analysis/src/TypeFunction.cpp
    crate::LUAU_DYNAMIC_FASTINTVARIABLE!(LuauTypeFamilyApplicationCartesianProductLimit, 5_000);
    // Analysis/src/TypeFunction.cpp
    crate::LUAU_DYNAMIC_FASTINTVARIABLE!(LuauTypeFamilyGraphReductionMaximumSteps, 1_000_000);
    // Analysis/src/TypeFunctionRuntimeBuilder.cpp
    crate::LUAU_DYNAMIC_FASTINTVARIABLE!(LuauTypeFunctionSerdeIterationLimit, 100_000);
    // Analysis/src/ConstraintGenerator.cpp
    crate::LUAU_DYNAMIC_FASTINTVARIABLE!(LuauConstraintGeneratorRecursionLimit, 300);
    // Analysis/src/Simplify.cpp
    crate::LUAU_DYNAMIC_FASTINTVARIABLE!(LuauSimplificationComplexityLimit, 8);
    // Analysis/src/BuiltinTypeFunctions.cpp
    crate::LUAU_DYNAMIC_FASTINTVARIABLE!(LuauStepRefineRecursionLimit, 64);
    // Analysis/src/Subtyping.cpp
    crate::LUAU_DYNAMIC_FASTINTVARIABLE!(LuauSubtypingRecursionLimit, 100);
    // Analysis/src/TypeFunction.cpp
    crate::LUAU_DYNAMIC_FASTINTVARIABLE!(LuauTypeFamilyUseGuesserDepth, -1);
    // Analysis/src/TypePath.cpp
    crate::LUAU_DYNAMIC_FASTINTVARIABLE!(LuauTypePathMaximumTraverseSteps, 100);
    // Analysis/src/Simplify.cpp
    crate::LUAU_DYNAMIC_FASTINTVARIABLE!(LuauTypeSimplificationIterationLimit, 128);
    // Analysis/src/Unifier2.cpp
    crate::LUAU_DYNAMIC_FASTINTVARIABLE!(LuauUnifierRecursionLimit, 100);
}

mod fastflag_timetrace_tests {
    /// The macro-defined flag reads its default; the TimeTrace consumer macros
    /// expand cleanly as no-ops (default `LUAU_ENABLE_TIME_TRACE` off).
    #[test]
    fn flag_default_and_timetrace_noops() {
        assert_eq!(crate::FFlag::DebugLuauTimeTracing.get(), false);
        crate::LUAU_TIMETRACE_SCOPE!("name", "category");
        crate::LUAU_TIMETRACE_OPTIONAL_TAIL_SCOPE!("name", "category", 100);
        crate::LUAU_TIMETRACE_ARGUMENT!("k", "v");
        crate::FFlag::DebugLuauTimeTracing.set(true);
        assert_eq!(crate::FFlag::DebugLuauTimeTracing.get(), true);
    }
}
