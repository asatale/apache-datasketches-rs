#include "dyn_summary.h"
#include "tuple_generic.rs.h" // the real declarations behind dyn_summary.h's forward declarations

// DynSummary is header-only: every member is defined inline, against nothing
// but a forward declaration of RustSummary (verified to compile and round-trip
// — rust::Box only stores a T*, so an incomplete T is fine for the member, the
// copy constructor, and std::optional). This translation unit exists to
// compile the header once on its own and to hold the static assertions below,
// and it is the one place that pulls in the generated header, proving the
// forward declarations above match it.

namespace apache_datasketches_rs {
namespace {

// Compile-time assertions on the properties upstream's entry table requires
// of a Summary. If any of these regress, the failure appears here rather than
// as an inscrutable template error inside datasketches-cpp.
static_assert(std::is_move_constructible<DynSummary>::value,
              "DynSummary must be move-constructible: the entry table "
              "move-constructs on every rehash and resize");
static_assert(std::is_nothrow_move_constructible<DynSummary>::value,
              "DynSummary must be NOTHROW-move-constructible: upstream's entry "
              "table move-constructs every entry on rehash and resize, and the "
              "explicit move constructor (which disengages the source to keep "
              "engaged() == get()-is-valid) must not weaken that");
static_assert(std::is_nothrow_move_assignable<DynSummary>::value,
              "DynSummary must be nothrow-move-assignable for the same reason");
static_assert(std::is_copy_constructible<DynSummary>::value,
              "DynSummary must be copy-constructible: the entry table "
              "copy-constructs when a whole sketch is copied");
static_assert(std::is_destructible<DynSummary>::value,
              "DynSummary must be destructible");
static_assert(std::is_default_constructible<DynSummary>::value,
              "DynSummary must be default-constructible: DynUpdatePolicy::create() "
              "returns a disengaged one");

} // namespace
} // namespace apache_datasketches_rs
