/// Critical Bands Theory Implementation for Dissonance Calculation
/// Based on Plomp & Levelt psychoacoustic research
use std::f32::consts::E;

// Reference octave for normalization (C4 = 261.63 Hz)
const REFERENCE_C: f32 = 261.63;

/// Calculate critical band width using Zwicker's formula
fn critical_band_width_zwicker(frequency_hz: f32) -> f32 {
    const A: f32 = 24.7;
    const B: f32 = 4.37;
    A * (B * frequency_hz / 1000.0 + 1.0)
}

/// Calculate dissonance between two pure tones using Plomp-Levelt model
fn dissonance_pure_tones(f1: f32, f2: f32) -> f32 {
    if f1 == f2 {
        return 0.0;
    }
    
    const DECAY_1: f32 = -3.5;
    const DECAY_2: f32 = -5.75;
    const FREQ_FACTOR: f32 = -0.25;
    
    let f_min = f1.min(f2);
    let f_max = f1.max(f2);
    let freq_diff = f_max - f_min;
    
    let cbw = critical_band_width_zwicker(f_min);
    let s = freq_diff / cbw;
    
    let dissonance = (E.powf(DECAY_1 * s) - E.powf(DECAY_2 * s)) * f_min.powf(FREQ_FACTOR);
    dissonance.max(0.0)
}

/// Convert semitones to frequency ratio
fn semitones_to_ratio(semitones: f32) -> f32 {
    2.0_f32.powf(semitones / 12.0)
}

/// Calculate frequencies for semitones in reference octave
fn semitones_to_frequencies(semitones: &[u8]) -> Vec<f32> {
    semitones.iter()
        .map(|&s| REFERENCE_C * semitones_to_ratio(s as f32))
        .collect()
}

/// Calculate dissonance for a set of notes (represented as semitones from C)
/// This version is octave-equivalent: different inversions have same dissonance
pub fn chord_dissonance(semitones: &[u8], max_harmonics: usize) -> f32 {
    // Handle special cases
    if semitones.is_empty() {
        return 0.0;
    }
    if semitones.len() == 1 {
        return 0.0; // Single note has no dissonance
    }
    
    // Normalize all notes to the same octave and remove duplicates
    let mut normalized_semitones: Vec<u8> = semitones.iter().map(|&s| s % 12).collect();
    normalized_semitones.sort_unstable();
    normalized_semitones.dedup();
    
    // If only one unique note remains, no dissonance
    if normalized_semitones.len() <= 1 {
        return 0.0;
    }
    
    // Convert to frequencies in reference octave
    let frequencies = semitones_to_frequencies(&normalized_semitones);
    
    // Generate harmonics for each frequency
    let mut all_components = Vec::new();
    for &freq in &frequencies {
        for harmonic in 1..=max_harmonics {
            let harmonic_freq = freq * harmonic as f32;
            let harmonic_amp = 1.0 / harmonic as f32; // 1/n amplitude rolloff
            all_components.push((harmonic_freq, harmonic_amp));
        }
    }
    
    // Calculate pairwise dissonances
    let mut total_dissonance = 0.0;
    for i in 0..all_components.len() {
        for j in (i + 1)..all_components.len() {
            let (f1, a1) = all_components[i];
            let (f2, a2) = all_components[j];
            
            let pair_dissonance = dissonance_pure_tones(f1, f2);
            total_dissonance += pair_dissonance * a1 * a2;
        }
    }
    
    total_dissonance
}

/// Calculate dissonance for a musical interval using critical bands theory
/// This version ensures that inversions have the same dissonance
pub fn interval_dissonance(semitones: u8) -> f32 {
    const MAX_HARMONICS: usize = 6;
    
    // Handle unison and octave specially
    if semitones % 12 == 0 {
        return 0.0;
    }
    
    // For inversion equivalence, always use the smaller interval
    // e.g., minor sixth (8 semitones) becomes major third (4 semitones)
    let normalized_semitones = semitones % 12;
    let smaller_interval = if normalized_semitones > 6 {
        12 - normalized_semitones
    } else {
        normalized_semitones
    };
    
    let semitones_vec = vec![0, smaller_interval]; // Root + smaller interval
    chord_dissonance(&semitones_vec, MAX_HARMONICS)
}

/// Normalization factor to scale critical bands values to match current system range
const NORMALIZATION_FACTOR: f32 = 15.0;

/// Calculate normalized dissonance for a musical interval
/// This matches the range of the previous hardcoded system (0.0 - 0.9)
pub fn interval_dissonance_normalized(semitones: u8) -> f32 {
    let raw_dissonance = interval_dissonance(semitones);
    (raw_dissonance * NORMALIZATION_FACTOR).min(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_interval_dissonances() {
        let intervals = [
            (0, "Unison"),
            (1, "Minor Second"), 
            (2, "Major Second"),
            (3, "Minor Third"),
            (4, "Major Third"),
            (5, "Perfect Fourth"),
            (6, "Tritone"),
            (7, "Perfect Fifth"),
            (8, "Minor Sixth"),
            (9, "Major Sixth"),
            (10, "Minor Seventh"),
            (11, "Major Seventh"),
        ];
        
        for (semitones, name) in intervals {
            let raw_dissonance = interval_dissonance(semitones);
            let normalized_dissonance = interval_dissonance_normalized(semitones);
            
            // Basic sanity checks
            assert!(raw_dissonance >= 0.0, "{name} should have non-negative dissonance");
            assert!(normalized_dissonance >= 0.0, "{name} normalized should be non-negative");
            assert!(normalized_dissonance <= 1.0, "{name} normalized should be <= 1.0");
            
            // Unison should have zero dissonance
            if semitones == 0 {
                assert_eq!(raw_dissonance, 0.0, "Unison should have zero dissonance");
            }
        }
    }
    
    #[test]
    fn test_chord_dissonance_octave_equivalence() {
        // Test that different inversions have the same dissonance
        let root_position = vec![0, 4, 7];        // C-E-G
        let first_inversion = vec![4, 7, 12];     // E-G-C (next octave)
        let second_inversion = vec![7, 12, 16];   // G-C-E (next octave)
        
        let root_dissonance = chord_dissonance(&root_position, 6);
        let first_dissonance = chord_dissonance(&first_inversion, 6);
        let second_dissonance = chord_dissonance(&second_inversion, 6);
        
        // All should be equal (within floating point precision)
        assert!((root_dissonance - first_dissonance).abs() < 1e-6);
        assert!((root_dissonance - second_dissonance).abs() < 1e-6);
    }
    
    #[test]
    fn test_duplicate_note_handling() {
        // Test that duplicate notes (same note in different octaves) are handled correctly
        let chord_with_duplicates = vec![0, 4, 7, 12]; // C-E-G-C (octave)
        let chord_without_duplicates = vec![0, 4, 7];   // C-E-G
        
        let dup_dissonance = chord_dissonance(&chord_with_duplicates, 6);
        let no_dup_dissonance = chord_dissonance(&chord_without_duplicates, 6);
        
        // Should be equal since duplicate is removed
        assert!((dup_dissonance - no_dup_dissonance).abs() < 1e-6);
    }
    
    #[test]
    fn test_empty_and_single_note() {
        // Empty chord should have zero dissonance
        assert_eq!(chord_dissonance(&[], 6), 0.0);
        
        // Single note should have zero dissonance
        assert_eq!(chord_dissonance(&[5], 6), 0.0);
    }
    
    #[test]
    fn test_most_dissonant_intervals() {
        // Test that minor second is the most dissonant interval
        let mut dissonances = Vec::new();
        for semitones in 0..=11 {
            dissonances.push((semitones, interval_dissonance(semitones)));
        }
        
        // Find the most dissonant interval
        let most_dissonant = dissonances.iter()
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .unwrap();
        
        // With inversion equivalence, both minor second (1) and major seventh (11) 
        // should have the same highest dissonance value
        let minor_second_dissonance = interval_dissonance(1);
        let major_seventh_dissonance = interval_dissonance(11);
        
        assert_eq!(minor_second_dissonance, major_seventh_dissonance, 
                   "Minor second and major seventh should have same dissonance (inversions)");
        
        assert_eq!(most_dissonant.1, minor_second_dissonance, 
                   "Most dissonant intervals should be minor second/major seventh pair");
    }
    
    #[test]
    fn test_inversion_equivalence() {
        // Test that inversions have the same dissonance
        let interval_pairs = [
            (1, 11),  // Minor second ↔ Major seventh
            (2, 10),  // Major second ↔ Minor seventh  
            (3, 9),   // Minor third ↔ Major sixth
            (4, 8),   // Major third ↔ Minor sixth
            (5, 7),   // Perfect fourth ↔ Perfect fifth
            // Tritone (6) is its own inversion
        ];
        
        for (interval1, interval2) in interval_pairs {
            let dissonance1 = interval_dissonance(interval1);
            let dissonance2 = interval_dissonance(interval2);
            
            assert!(
                (dissonance1 - dissonance2).abs() < 1e-6,
                "Inversion pair {interval1} and {interval2} should have same dissonance: {dissonance1:.6} vs {dissonance2:.6}"
            );
        }
        
        // Test tritone with itself
        let tritone_dissonance = interval_dissonance(6);
        assert!(tritone_dissonance > 0.0, "Tritone should have non-zero dissonance");
        
        // Test unison and octave
        assert_eq!(interval_dissonance(0), 0.0, "Unison should have zero dissonance");
        assert_eq!(interval_dissonance(12), 0.0, "Octave should have zero dissonance");
    }
}
