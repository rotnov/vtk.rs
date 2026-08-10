#include <valgrind/callgrind.h>

#include <cstdint>
#include <cstdio>
#include <optional>
#include <vector>

namespace {

std::vector<double> generate_coords(std::size_t n_points) {
  std::vector<double> coords;
  coords.reserve(n_points * 3);
  for (std::uint64_t i = 0; i < static_cast<std::uint64_t>(n_points * 3); ++i) {
    std::uint64_t bits = i * 2654435761ULL;
    coords.push_back(static_cast<double>(bits % 100000ULL) / 1000.0);
  }
  return coords;
}

struct Bounds {
  double values[6];
};

// Equivalent of Points::bounds() in rust/spike-numeric-array/src/points.rs — acquires nothing
// (no lock in the C++ reference; VTK's own vtkPoints::GetBounds() doesn't lock either), matches
// the Rust kernel's single pass over the flat xyz buffer.
std::optional<Bounds> bounds_of_points(const std::vector<double>& xyz) {
  if (xyz.empty() || xyz.size() % 3 != 0) {
    return std::nullopt;
  }
  Bounds b{{xyz[0], xyz[0], xyz[1], xyz[1], xyz[2], xyz[2]}};
  for (std::size_t i = 3; i < xyz.size(); i += 3) {
    double x = xyz[i];
    double y = xyz[i + 1];
    double z = xyz[i + 2];
    if (x < b.values[0]) b.values[0] = x;
    if (x > b.values[1]) b.values[1] = x;
    if (y < b.values[2]) b.values[2] = y;
    if (y > b.values[3]) b.values[3] = y;
    if (z < b.values[4]) b.values[4] = z;
    if (z > b.values[5]) b.values[5] = z;
  }
  return b;
}

}  // namespace

int main() {
  constexpr std::size_t kNumPoints = 1'000'000;
  std::vector<double> coords = generate_coords(kNumPoints);

  CALLGRIND_TOGGLE_COLLECT;
  std::optional<Bounds> result = bounds_of_points(coords);
  CALLGRIND_TOGGLE_COLLECT;

  if (result) {
    std::printf("bounds: %f %f %f %f %f %f\n", result->values[0], result->values[1],
                result->values[2], result->values[3], result->values[4], result->values[5]);
  }
  return 0;
}
